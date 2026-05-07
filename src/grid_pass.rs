//! Infinity grid as a `CustomRenderPass` at the `Scene3D` stage.
//!
//! Renders an anti-aliased ground-plane grid using analytical ray-plane
//! intersection in the fragment shader. No geometry — just a fullscreen
//! triangle. The pass uses the `inv_view_proj` and `camera_pos` from
//! `RenderPassContext` (populated for `Scene3D` stage passes).

use blinc_gpu::custom_pass::{CustomRenderPass, RenderPassContext, RenderStage};

const GRID_SHADER: &str = include_str!("shaders/grid.wgsl");

pub struct GridPass {
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    uniform_buffer: Option<wgpu::Buffer>,
    pub grid_size: f32,
    pub subdivisions: f32,
    pub fade_near: f32,
    pub fade_far: f32,
    pub thin_color: [f32; 4],
    pub thick_color: [f32; 4],
    enabled: bool,
}

impl Default for GridPass {
    fn default() -> Self {
        Self::new()
    }
}

impl GridPass {
    pub fn new() -> Self {
        Self {
            pipeline: None,
            bind_group_layout: None,
            uniform_buffer: None,
            grid_size: 1.0,
            subdivisions: 5.0,
            fade_near: 8.0,
            fade_far: 30.0,
            thin_color: [0.3, 0.3, 0.3, 0.3],
            thick_color: [0.5, 0.5, 0.5, 0.5],
            enabled: true,
        }
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.grid_size = size;
        self
    }

    pub fn with_subdivisions(mut self, sub: u32) -> Self {
        self.subdivisions = sub as f32;
        self
    }

    pub fn with_fade(mut self, near: f32, far: f32) -> Self {
        self.fade_near = near;
        self.fade_far = far;
        self
    }
}

impl CustomRenderPass for GridPass {
    fn label(&self) -> &str {
        "Infinity Grid"
    }

    fn stage(&self) -> RenderStage {
        RenderStage::Scene3D
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn initialize(
        &mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _format: wgpu::TextureFormat,
    ) {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Grid Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Scene3D passes render to the Rgba16Float HDR intermediate,
        // not the surface format. Hardcode to match.
        let pipeline = blinc_gpu::custom_pass::create_fullscreen_pipeline(
            device,
            "Grid Pipeline",
            GRID_SHADER,
            wgpu::TextureFormat::Rgba16Float,
            &bind_group_layout,
        );

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Grid Uniforms"),
            size: 128,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.pipeline = Some(pipeline);
        self.bind_group_layout = Some(bind_group_layout);
        self.uniform_buffer = Some(uniform_buffer);
    }

    fn render(&mut self, ctx: &RenderPassContext) {
        let (Some(pipeline), Some(layout), Some(buffer)) = (
            &self.pipeline,
            &self.bind_group_layout,
            &self.uniform_buffer,
        ) else {
            return;
        };

        let (Some(inv_vp), Some(cam_pos)) = (ctx.inv_view_proj, ctx.camera_pos) else {
            return;
        };

        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct GridUniforms {
            inv_view_proj: [f32; 16],
            camera_pos: [f32; 3],
            grid_size: f32,
            thin_color: [f32; 4],
            thick_color: [f32; 4],
            fade_near: f32,
            fade_far: f32,
            subdivisions: f32,
            _pad: f32,
        }

        let uniforms = GridUniforms {
            inv_view_proj: inv_vp,
            camera_pos: cam_pos,
            grid_size: self.grid_size,
            thin_color: self.thin_color,
            thick_color: self.thick_color,
            fade_near: self.fade_near,
            fade_far: self.fade_far,
            subdivisions: self.subdivisions,
            _pad: 0.0,
        };

        ctx.queue
            .write_buffer(buffer, 0, bytemuck::bytes_of(&uniforms));

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Grid Bind Group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Grid Encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Grid Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: ctx.target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            pass.set_pipeline(pipeline);

            // Clip to the canvas viewport so the grid aligns with the
            // mesh (which also renders at this viewport offset).
            if let Some([vx, vy, vw, vh]) = ctx.viewport {
                pass.set_viewport(vx, vy, vw.max(1.0), vh.max(1.0), 0.0, 1.0);
                pass.set_scissor_rect(
                    vx.max(0.0) as u32,
                    vy.max(0.0) as u32,
                    vw.max(1.0) as u32,
                    vh.max(1.0) as u32,
                );
            }

            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        ctx.queue.submit(std::iter::once(encoder.finish()));
    }
}
