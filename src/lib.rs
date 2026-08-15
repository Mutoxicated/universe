pub mod game;
mod mansel;
mod render;
mod state;
pub mod statics;
pub mod utils;
pub use utils::MutPtr;

use game::ToGame;
use state::State;
use std::sync::mpsc::{self, Sender};

use winit::{
    application::ApplicationHandler,
    event::KeyEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::WindowAttributes,
};

use crate::{game::GameCallbacks, render::RenderBatch};

pub fn let_there_be_light(g: game::GameSimulation, callbacks: Box<dyn GameCallbacks>) {
    let e = EventLoop::<ToMainframe>::with_user_event().build().unwrap();
    e.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let (s, r) = mpsc::channel::<game::ToGame>();
    let proxy = e.create_proxy();

    let mut p = Program::new(s);
    std::thread::Builder::new()
        .name(String::from("game thread"))
        .spawn(move || g.simulate(r, proxy, callbacks))
        .unwrap();
    e.run_app(&mut p).unwrap();
}

pub enum ToMainframe {
    RenderRequest(render::RenderBatch),
    PoolMeshRequest(render::Mesh),
}

pub enum GPUResources {
    VertexAndIndexBuffer {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        length: u32,
    },
    VertexBuffer {
        vb: wgpu::Buffer,
        length: u32,
    },
}

struct Program {
    state: Option<State>,
    to_game: Sender<ToGame>,
    render_batch: RenderBatch,
}

impl Program {
    fn new(to_game: Sender<ToGame>) -> Program {
        Self {
            state: None,
            to_game,
            render_batch: RenderBatch::empty(),
        }
    }
}

impl ApplicationHandler<ToMainframe> for Program {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(WindowAttributes::default().with_title("Powered by Gooberverse"))
            .unwrap();
        let size = window.inner_size();
        self.state = Some(State::new(window));
        let _ = self.to_game.send(ToGame::Start);
        let _ = self.to_game.send(ToGame::UpdateCameraAspectRatio(
            size.width as f32 / size.height as f32,
        ));
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
                let _ = self.to_game.send(ToGame::Stop);
            }
            W::Resized(size) => {
                state.resize(size.width, size.height);
                let _ = self.to_game.send(ToGame::UpdateCameraAspectRatio(
                    size.width as f32 / size.height as f32,
                ));
            }
            W::RedrawRequested => {
                state.render(&self.render_batch, 0.0);
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: ToMainframe) {
        use ToMainframe as M;
        match event {
            M::RenderRequest(b) => {
                self.render_batch = b;
            }
            M::PoolMeshRequest(mesh) => {
                let state = self.state.as_mut().unwrap();
                state.pool_gpu_resources_from_mesh(mesh);
            }
        }
    }
}
