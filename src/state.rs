use std::sync::Arc;

use wgpu::{
    DeviceDescriptor, ExperimentalFeatures, Features, InstanceDescriptor, Limits, util::DeviceExt,
};
use winit::window::Window;

use crate::{
    GPUResources,
    render::{
        RenderBatch,
        RenderObject::{PooledInstance, SingleInstance},
        Vertex,
    },
};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    render_pipeline: wgpu::RenderPipeline,
    pooled_gpu_resources: Vec<(Arc<str>, GPUResources)>,
    camera_uniform: render::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
}

impl State {
    pub fn new(window: Window) -> State {
        let window = Arc::new(window);
        let instance = wgpu::Instance::new(InstanceDescriptor::new_without_display_handle());

        let surface = instance.create_surface(window.clone()).unwrap();

        let fut = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: true,
            compatible_surface: Some(&surface),
            apply_limit_buckets: true,
        });
        let adapter = pollster::block_on(fut).unwrap();

        let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
            label: None,
            required_features: Features::empty(),
            experimental_features: ExperimentalFeatures::disabled(),
            required_limits: Limits::defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let w_size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: w_size.width,
            height: w_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let mut camera_uniform = CameraUniform::new();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("Camera Bind Group Layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[None, Some(&camera_bind_group_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::desc())],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            window,
            render_pipeline,
            pooled_gpu_resources: vec![],
            camera_uniform,
            camera_buffer,
            camera_bind_group,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self, batch: &RenderBatch, ival: f32) {
        self.window.request_redraw();
        use wgpu::CurrentSurfaceTexture as T;
        let output = match self.surface.get_current_texture() {
            T::Success(t) => t,
            T::Suboptimal(t) => t,
            T::Timeout | T::Occluded | T::Validation => return,
            T::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            T::Lost => {
                panic!("Lost device!")
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        self.camera_uniform.update(batch.camera);
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        for o in batch.objects.iter() {
            match o {
                PooledInstance {
                    previous_frame,
                    current_frame,
                    mesh_name,
                } => {
                    let gpur = self.find_gpu_resources(mesh_name.clone());
                    self.draw_mesh_from_gpu_resources(&mut render_pass, gpur);
                }
                SingleInstance {
                    previous_frame: _,
                    current_frame: _,
                    mesh: _,
                } => {}
            }
        }
        drop(render_pass);
        self.window.pre_present_notify();
        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    fn find_gpu_resources(&self, mesh_name: Arc<str>) -> &GPUResources {
        &self
            .pooled_gpu_resources
            .iter()
            .find(|a| a.0 == mesh_name)
            .unwrap()
            .1
    }

    fn draw_mesh_from_gpu_resources(
        &self,
        render_pass: &mut wgpu::RenderPass,
        gpur: &GPUResources,
    ) {
        use GPUResources as G;
        match gpur {
            G::VertexAndIndexBuffer { vb, ib, length } => {
                render_pass.set_vertex_buffer(0, vb.slice(..));
                render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..*length, 0, 0..1);
            }
            G::VertexBuffer { vb, length } => {
                render_pass.set_vertex_buffer(0, vb.slice(..));
                render_pass.draw(0..*length, 0..1);
            }
        }
    }

    pub fn pool_gpu_resources_from_mesh(&mut self, mesh: crate::render::Mesh) {
        let gpur = self.create_gpu_resources_from_mesh(&mesh);
        self.pooled_gpu_resources.push((mesh.name.clone(), gpur));
    }

    pub fn create_gpu_resources_from_mesh(&self, mesh: &crate::render::Mesh) -> GPUResources {
        let vb = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ib = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        GPUResources::VertexAndIndexBuffer {
            vb,
            ib,
            length: mesh.indices.len() as u32,
        }
    }
}
