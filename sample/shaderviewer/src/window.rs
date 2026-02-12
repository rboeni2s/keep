use winit::{
    application::ApplicationHandler,
    event::WindowEvent as PlatformWindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window as PlatformWindow, WindowAttributes, WindowId},
};

use crate::service::{REGISTRY, application::ApplicationEvent, renderer::Renderer};
use plug::prelude::*;


#[derive(Debug, Clone, Copy)]
pub enum WindowEvent
{
    Close,
    Resize(u32, u32),
    Redraw,
}


pub struct WindowHandler
{
    window: Option<Guard<PlatformWindow>>,
    window_event_emitter: Layer<EventEmitter<WindowEvent>>,
    latest_resize_event: Option<(u32, u32)>,
}


impl WindowHandler
{
    pub fn new(window_event_emitter: Layer<EventEmitter<WindowEvent>>) -> Self
    {
        Self {
            window: None,
            latest_resize_event: None,
            window_event_emitter,
        }
    }
}


impl ApplicationHandler<ApplicationEvent> for WindowHandler
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop)
    {
        match event_loop.create_window(WindowAttributes::default())
        {
            Ok(window) =>
            {
                let window = Guard::new(window);
                self.window = Some(window.clone());

                if let Some(reg) = REGISTRY.get()
                {
                    match reg.get::<Renderer>()
                    {
                        Some(renderer) =>
                        {
                            match renderer.init_from_window(window)
                            {
                                Ok(_) => info!("Renderer initialized"),
                                Err(e) =>
                                {
                                    error!("Failed to initialize renderer: {e}");
                                    self.window_event_emitter.emit(WindowEvent::Close);
                                }
                            }
                        }

                        None =>
                        {
                            error!("Failed to initialize renderer: Renderer does not exist");
                            self.window_event_emitter.emit(WindowEvent::Close);
                        }
                    }
                }
            }

            Err(e) =>
            {
                error!("Window creation failed: {e}");
                self.window_event_emitter.emit(WindowEvent::Close);
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: PlatformWindowEvent,
    )
    {
        match event
        {
            PlatformWindowEvent::CloseRequested =>
            {
                info!("WindowClose requested");
                self.window_event_emitter.emit(WindowEvent::Close);
            }

            PlatformWindowEvent::RedrawRequested =>
            {
                self.window_event_emitter.emit(WindowEvent::Redraw);
            }

            PlatformWindowEvent::Resized(size) =>
            {
                self.latest_resize_event = Some((size.width, size.height))
            }

            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ApplicationEvent)
    {
        match event
        {
            ApplicationEvent::Close => event_loop.exit(),
            ApplicationEvent::TickWindow =>
            {
                if let Some((w, h)) = self.latest_resize_event.take()
                    && self.window.is_some()
                {
                    self.window_event_emitter.emit(WindowEvent::Resize(w, h));
                }
            }
            _ => (),
        }
    }
}
