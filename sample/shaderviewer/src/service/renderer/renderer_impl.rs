use anyhow::Context;
use keep::Guard;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window as PlatformWindow;


pub struct RenderState
{
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: Mutex<wgpu::SurfaceConfiguration>,
    surface_out_of_date: AtomicBool,
    window: Guard<PlatformWindow>,
}


/// Wrapper struct to extract the window and display handle from `Guard<PlatformWindow>`
struct WindowWrapper(Guard<PlatformWindow>);
impl HasDisplayHandle for WindowWrapper
{
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError>
    {
        self.0.display_handle()
    }
}

impl HasWindowHandle for WindowWrapper
{
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError>
    {
        self.0.window_handle()
    }
}


impl RenderState
{
    /// Initializes a wgpu backend for `window`
    pub async fn new(window: Guard<PlatformWindow>) -> anyhow::Result<Self>
    {
        let surface_size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface_target = wgpu::SurfaceTarget::from(WindowWrapper(window.clone()));
        let surface = instance.create_surface(surface_target)?;

        // Select a adapter. Try to get a adapter for a discrete gpu first
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
        {
            Ok(adapter) => adapter,

            // If the request didn't work, select any available adapter that supports our surface
            Err(e) =>
            {
                warn!("Failed to select preferred GPU adapter: {e}");

                instance
                    .enumerate_adapters(wgpu::Backends::all())
                    .await
                    .into_iter()
                    .find(|a| a.is_surface_supported(&surface))
                    .context("No supported GPU adapter found")?
            }
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
                ..Default::default()
            })
            .await?;

        // Select a sRGB surface format, fallback to any available format if srgb is not available
        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(
                surface_capabilities
                    .formats
                    .first()
                    .copied()
                    .context("No surface formats available")?,
            );

        // Select a present mode based on the following priorities:
        // * 1. Mailbox
        // * 2. FiFo
        // * 3. Any present mode
        let present_mode = {
            let mut present_mode = surface_capabilities
                .present_modes
                .first() // Use the first available format as fallback if neither mailbox nor fifo are available
                .copied()
                .context("No available present modes")?;

            let mut fifo_available = false;

            for mode in surface_capabilities.present_modes
            {
                match mode
                {
                    wgpu::PresentMode::Mailbox =>
                    {
                        present_mode = wgpu::PresentMode::Mailbox;
                        break;
                    }

                    wgpu::PresentMode::AutoVsync
                    | wgpu::PresentMode::Fifo
                    | wgpu::PresentMode::FifoRelaxed => fifo_available = true,

                    _ => (),
                }
            }

            if fifo_available && present_mode != wgpu::PresentMode::Mailbox
            {
                present_mode = wgpu::PresentMode::AutoVsync;
            }

            present_mode
        };

        // Select opaque alpha compositing mode, inherit otherwise
        let alpha_mode = surface_capabilities
            .alpha_modes
            .iter()
            .find(|a| **a == wgpu::CompositeAlphaMode::Opaque)
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Inherit);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: surface_size.width.max(1), // Ensure that the surface width and height are always larger than 0
            height: surface_size.height.max(1),
            present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode,
            view_formats: vec![],
        };

        let info = adapter.get_info();

        info!("Initialized renderer for adapter:\n{info:#?}");

        let me = Self {
            surface,
            device,
            queue,
            config: Mutex::new(surface_config),
            surface_out_of_date: AtomicBool::new(true),
            window,
        };

        // Finally perform a initial window resize
        me.resize(surface_size.width, surface_size.height)?;

        Ok(me)
    }

    pub fn resize(&self, width: u32, height: u32) -> anyhow::Result<()>
    {
        if width == 0 || height == 0
        {
            return Err(anyhow::Error::msg(format!(
                "Invalid width({width}) and/or height({height})"
            )));
        }

        let mut config = self.config.lock().unwrap();
        config.width = width;
        config.height = height;
        self.surface.configure(&self.device, &config);
        self.surface_out_of_date.store(false, Ordering::Release);

        info!("Surface resize finished ({width}, {height})");

        Ok(())
    }

    pub fn render(&self) -> anyhow::Result<()>
    {
        self.window.request_redraw();

        if self.surface_out_of_date.load(Ordering::Acquire)
        {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        drop(_render_pass);
        self.queue.submit([encoder.finish()]);
        output.present();

        Ok(())
    }
}
