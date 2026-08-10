mod camera;
pub use camera::{Camera, ViewFrustum};
use glam::Vec3;
use std::{sync::mpsc, thread, time::Duration};

use crate::ToMainframe;

mod handle_code {
    pub static EXIT: i8 = -1;
    pub static ALL_GOOD: i8 = 0;
}

pub struct Physics {
    /// in milliseconds
    tick_rate: u64,
    gravity: Vec3,
}

impl Physics {
    pub fn new(tick_rate_millis: u64, gravity: Vec3) -> Self {
        Self {
            tick_rate: tick_rate_millis,
            gravity,
        }
    }
}

pub enum ToGame {
    STOP,
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
        _to_mainframe: mpsc::Sender<ToMainframe>,
        callbacks: &'static mut dyn GameCallbacks,
    ) {
        let tick = self.physics.tick_rate;
        let dt: f32 = tick as f32 / 1000.0;

        callbacks.start(&mut self);
        'a: loop {
            thread::sleep(Duration::from_millis(tick));

            let msgs = mainframe.try_iter();
            for msg in msgs {
                if self.handle_mainframe(msg) == handle_code::EXIT {
                    break 'a;
                }
            }

            callbacks.update(&mut self, dt);
        }

        callbacks.exit(&mut self);
    }

    fn handle_mainframe(&mut self, msg: ToGame) -> i8 {
        use ToGame as G;
        match msg {
            G::STOP => return handle_code::EXIT,
        };
    }
}
