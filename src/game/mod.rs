mod camera;
mod object;
mod physics;
pub use camera::{Camera, ViewFrustum};
pub use object::{Custom, Object};
pub use physics::Physics;
use std::{sync::mpsc, thread, time::Duration};
use winit::event_loop::EventLoopProxy;

use crate::ToMainframe;

mod handle_code {
    pub static EXIT: i8 = -1;
    pub static ALL_GOOD: i8 = 0;
}

#[derive(PartialEq)]
pub enum ToGame {
    Stop,
    /// Tells the game thread that the main thread has started
    /// and initialized the window so that game thread can now start
    Start,
    UpdateCameraAspectRatio(f32),
}

pub trait GameCallbacks: Send + Sync {
    fn start(&mut self, game: &mut GameSimulation);
    fn update(&mut self, game: &mut GameSimulation, dt: f32);
    fn exit(&mut self, game: &mut GameSimulation);
}

pub struct GameSimulation {
    camera: Camera,
    physics: Physics,

    objects: Vec<Object>,
    customs: Vec<Box<dyn Custom>>,
    objects_life: Vec<bool>,
}

impl GameSimulation {
    pub fn new(camera: Camera, physics: Physics) -> GameSimulation {
        Self {
            camera,
            physics,
            objects: Vec::new(),
            customs: Vec::new(),
            objects_life: Vec::new(),
        }
    }

    pub fn register_object(&mut self, object: Object, custom: Box<dyn Custom>) {
        self.objects.push(object);
        self.customs.push(custom);
        self.objects_life.push(true);
    }

    pub fn kill_object(&mut self, index: usize) {
        self.objects_life[index] = false;
    }

    pub(crate) fn simulate(
        mut self,
        mainframe: mpsc::Receiver<ToGame>,
        to_mainframe: EventLoopProxy<ToMainframe>,
        mut callbacks: Box<dyn GameCallbacks>,
    ) {
        let tick = self.physics.tick_rate;
        let dt: f32 = tick as f32 / 1000.0;

        let msg = mainframe.recv();
        if msg.is_err() || msg.unwrap() != ToGame::Start {
            return;
        }
        callbacks.start(&mut self);
        let mut index = self.objects.len();
        while index > 0 {
            index -= 1;
            let mut custom = self.customs.remove(index);
            let mut object = self.objects.remove(index);
            custom.start(&mut object, index, &mut self);
            let alive = self.objects_life.remove(index);
            if !alive {
                continue;
            }
            self.objects.push(object);
            self.customs.push(custom);
            self.objects_life.push(true);
        }

        'a: loop {
            thread::sleep(Duration::from_millis(tick));

            let msgs = mainframe.try_iter();
            for msg in msgs {
                if self.handle_mainframe(msg) == handle_code::EXIT {
                    break 'a;
                }
            }

            callbacks.update(&mut self, dt);

            let mut index = self.objects.len();
            while index > 0 {
                index -= 1;
                let mut custom = self.customs.remove(index);
                let mut object = self.objects.remove(index);
                custom.update(&mut object, index, &mut self, dt);
                let alive = self.objects_life.remove(index);
                if !alive {
                    continue;
                }
                self.objects.push(object);
                self.customs.push(custom);
                self.objects_life.push(true);
            }
        }

        callbacks.exit(&mut self);
    }

    fn handle_mainframe(&mut self, msg: ToGame) -> i8 {
        use ToGame as G;
        match msg {
            G::Stop => return handle_code::EXIT,
            G::Start => return handle_code::ALL_GOOD,
            G::UpdateCameraAspectRatio(ratio) => {
                self.camera.update_aspect_ratio(ratio);
                return handle_code::ALL_GOOD;
            }
        }
    }
}
