mod camera;
mod physics;
pub use camera::{Camera, ViewFrustum};
use glam::{Quat, Vec3};
pub use physics::Physics;
use std::{ops::Index, sync::mpsc, thread, time::Duration};
use winit::event_loop::EventLoopProxy;

use crate::{
    ToMainframe,
    render::{FrameObjectData, Mesh, RenderBatch, RenderObject, Vertex},
};

mod handle_code {
    pub static EXIT: i8 = -1;
    pub static START: i8 = 1;
    pub static ALL_GOOD: i8 = 0;
}

#[derive(PartialEq, Eq)]
pub enum ToGame {
    STOP,
    /// Tells the game thread that the main thread has started
    /// and initialized the window so that game thread can now start
    START,
    UPDATE_CAMERA_ASPECT_RATIO(f32),
}

pub trait GameCallbacks: Send + Sync {
    fn start(&mut self, game: &mut GameSimulation);
    fn update(&mut self, game: &mut GameSimulation, dt: f32);
    fn exit(&mut self, game: &mut GameSimulation);
}

pub struct GameSimulation {
    camera: Camera,
    physics: Physics,
}

impl GameSimulation {
    pub fn new(camera: Camera, physics: Physics) -> GameSimulation {
        Self { camera, physics }
    }

    pub(crate) fn simulate(
        mut self,
        mainframe: mpsc::Receiver<ToGame>,
        to_mainframe: EventLoopProxy<ToMainframe>,
        mut callbacks: Box<dyn GameCallbacks>,
    ) {
        let ofd = FrameObjectData {
            position: Vec3::new(0.0, 0.0, 10.0),
            rotation: Quat::IDENTITY,
        };
        let mesh = Mesh {
            vertices: vec![
                Vertex {
                    position: [0.0, 0.5, 0.0],
                    color: [1.0, 0.0, 0.0, 1.0],
                },
                Vertex {
                    position: [-0.5, -0.5, 0.0],
                    color: [0.0, 1.0, 0.0, 1.0],
                },
                Vertex {
                    position: [0.5, -0.5, 0.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                },
            ],
            indices: vec![0, 1, 2],
            name: "cube".into(),
        };

        let tick = self.physics.tick_rate;
        let dt: f32 = tick as f32 / 1000.0;

        callbacks.start(&mut self);
        let msg = mainframe.recv();
        if msg.is_err() || msg.unwrap() != ToGame::START {
            return;
        }

        let _ = to_mainframe.send_event(ToMainframe::PoolMeshRequest(mesh.clone()));

        'a: loop {
            thread::sleep(Duration::from_millis(tick));

            let msgs = mainframe.try_iter();
            for msg in msgs {
                if self.handle_mainframe(msg) == handle_code::EXIT {
                    break 'a;
                }
            }

            callbacks.update(&mut self, dt);

            let res = to_mainframe.send_event(ToMainframe::RenderRequest(RenderBatch {
                camera: self.camera.clone(),
                objects: vec![RenderObject::PooledInstance {
                    previous_frame: ofd.clone(),
                    current_frame: ofd.clone(),
                    mesh_name: mesh.name.clone(),
                }]
                .into(),
            }));
            if res.is_err() {
                break;
            }
        }

        callbacks.exit(&mut self);
    }

    fn handle_mainframe(&mut self, msg: ToGame) -> i8 {
        use ToGame as G;
        match msg {
            G::STOP => return handle_code::EXIT,
            G::START => return handle_code::ALL_GOOD,
            G::UPDATE_CAMERA_ASPECT_RATIO(ratio) => self.camera.update_aspect_ratio(ratio),
        };
    }
}
