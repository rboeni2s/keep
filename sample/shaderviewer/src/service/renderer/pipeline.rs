use wgpu::VertexBufferLayout;


pub(crate) struct Pipeline
{
    pub pipeline: wgpu::RenderPipeline,
}


impl Pipeline
{
    pub const TEST_SHADER_SRC: &str = include_str!(crate::rel!("/shaders/test.wgsl"));

    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        vertex_buffer_format: VertexBufferLayout,
    ) -> anyhow::Result<Self>
    {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Test Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::TEST_SHADER_SRC.into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let vertex_state = wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[vertex_buffer_format],
        };

        let fragment_state = wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        };

        let primitive_state = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        };

        let multi_sample_state = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render-Pipeline"),
            layout: Some(&layout),
            vertex: vertex_state,
            primitive: primitive_state,
            depth_stencil: None,
            multisample: multi_sample_state,
            fragment: Some(fragment_state),
            multiview_mask: None,
            cache: None,
        });

        Ok(Self { pipeline })
    }

    pub fn set_for_pass(&self, render_pass: &mut wgpu::RenderPass)
    {
        render_pass.set_pipeline(&self.pipeline);
    }
}
