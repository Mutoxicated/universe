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

pub trait GameCallbacks: Sync {
    fn start(&mut self, game: &mut GameSimulation);
    fn update(&mut self, game: &mut GameSimulation, dt: f32);
    fn exit(&mut self, game: &mut GameSimulation);
}

pub struct GameSimulation {
    camera: Camera,
    physics: Physics,
    callbacks: &'static dyn GameCallbacks,
}

impl GameSimulation {
    /// By creating a game simulation, you are promising that
    /// the object `callbacks` will only ever be borrowed ONCE
    /// and that borrow belongs to the game simulation.
    ///
    /// See [this](https://doc.rust-lang.org/reference/items/static-items.html#r-items.static.mut.intro)
    pub fn new(
        camera: Camera,
        physics: Physics,
        callbacks: &'static dyn GameCallbacks,
    ) -> GameSimulation {
        Self {
            camera,
            physics,
            callbacks,
        }
    }

    pub(crate) fn simulate(
        mut self,
        mainframe: mpsc::Receiver<ToGame>,
        _to_mainframe: mpsc::Sender<ToMainframe>,
    ) {
        let tick = self.physics.tick_rate;

        'a: loop {
            thread::sleep(Duration::from_millis(tick));

            let msgs = mainframe.try_iter();
            for msg in msgs {
                if self.handle_mainframe(msg) == handle_code::EXIT {
                    break 'a;
                }
            }
        }
    }

    fn handle_mainframe(&mut self, msg: ToGame) -> i8 {
        use ToGame as G;
        match msg {
            G::STOP => return handle_code::EXIT,
        };
    }
}
