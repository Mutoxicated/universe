use std::sync::Arc;

use wgpu::{DeviceDescriptor, ExperimentalFeatures, Features, InstanceDescriptor, Limits};
use winit::{
    application::ApplicationHandler,
    dpi::Size,
    event::KeyEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    platform::x11::WindowAttributesExtX11,
    window::{Cursor, Theme, Window, WindowAttributes, WindowButtons, WindowLevel},
};

pub struct ProgramRunner {}

impl ProgramRunner {
    pub fn action() {
        let e = EventLoop::<State>::with_user_event().build().unwrap();
        let mut p = Program::new();
        e.run_app(&mut p).unwrap();
    }
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
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
        Self {
            surface,
            device,
            queue,
            config,
            window,
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

    pub fn render(&mut self) {
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

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);
    }
}

struct Program {
    state: Option<State>,
}

impl Program {
    fn new() -> Program {
        Self { state: None }
    }
}

impl ApplicationHandler<State> for Program {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("Powered by Gooberverse"))
            .unwrap();
        self.state = Some(State::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(w) => w,
            None => return,
        };

        use winit::event::WindowEvent as W;
        use winit::keyboard::KeyCode as K;

        match event {
            W::CloseRequested => {
                event_loop.exit();
            }
            W::Resized(size) => {
                state.resize(size.width, size.height);
            }
            W::RedrawRequested => {
                state.render();
            }
            W::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => match (code, key_state.is_pressed()) {
                (K::Escape, true) => event_loop.exit(),
                _ => {}
            },
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: State) {
        self.state = Some(event)
    }
}
