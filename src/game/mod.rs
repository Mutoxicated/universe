mod camera;
mod object;
mod physics;
pub use camera::{Camera, ViewFrustum};
pub use object::{Custom, Object};
pub use physics::Physics;
use std::{
    collections::HashMap,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};
use winit::event_loop::EventLoopProxy;

use crate::{
    ToMainframe,
    game::object::ObjectInfo,
    mansel::{
        Poll::{self, Ready},
        Task,
    },
    render::{FrameObjectData, IPooledInstance, Mesh, RenderObject, load_gltf_into_mesh},
};

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

    object_frames: Vec<FrameObjectData>,
    objects: Vec<Object>,
    customs: Vec<Box<dyn Custom>>,
    object_infos: Vec<ObjectInfo>,
    registered_meshes: HashMap<Arc<str>, bool>,

    mesh_load_tasks: Vec<Task<Result<Mesh, Arc<str>>>>,
}

impl GameSimulation {
    pub fn new(camera: Camera, physics: Physics) -> GameSimulation {
        Self {
            camera,
            physics,
            object_frames: Vec::new(),
            objects: Vec::new(),
            customs: Vec::new(),
            object_infos: Vec::new(),
            registered_meshes: HashMap::new(),
            mesh_load_tasks: Vec::new(),
        }
    }

    pub fn pool_object_mesh(&mut self, mesh_path: Arc<str>) {
        self.registered_meshes.insert(mesh_path.clone(), false);
        self.mesh_load_tasks
            .push(crate::mansel::Task::spawn(move || {
                if mesh_path.ends_with(".gltf") {
                    load_gltf_into_mesh(mesh_path.clone())
                } else {
                    Err("universe only supports .gltf format for meshes as of now!".into())
                }
            }));
    }

    pub fn register_object(&mut self, object: Object, custom: Box<dyn Custom>) {
        if !self.registered_meshes.contains_key(&object.mesh_path) {
            self.pool_object_mesh(object.mesh_path.clone());
        }
        self.object_infos.push(ObjectInfo { is_alive: true });
        self.object_frames.push(FrameObjectData {
            position: object.position,
            scale: object.scale,
            rotation: object.rotation,
        });
        self.objects.push(object);
        self.customs.push(custom);
    }

    pub fn kill_object(&mut self, index: usize) {
        self.object_infos[index].is_alive = false;
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
            let info = self.object_infos.remove(index);
            if !info.is_alive {
                continue;
            }
            self.objects.push(object);
            self.customs.push(custom);
            self.object_infos.push(info);
        }
        self.check_mesh_load_tasks(&to_mainframe);

        'a: loop {
            thread::sleep(Duration::from_millis(tick));

            let msgs = mainframe.try_iter();
            for msg in msgs {
                if self.handle_mainframe(msg) == handle_code::EXIT {
                    break 'a;
                }
            }

            callbacks.update(&mut self, dt);
            self.check_mesh_load_tasks(&to_mainframe);

            let mut ipi = Vec::<IPooledInstance>::with_capacity(self.objects.len());
            let mut index = self.objects.len();
            while index > 0 {
                index -= 1;
                let mut custom = self.customs.swap_remove(index);
                let mut object = self.objects.swap_remove(index);
                let ofd = self.object_frames.swap_remove(index);
                custom.update(&mut object, index, &mut self, dt);
                let info = self.object_infos.swap_remove(index);
                if !info.is_alive {
                    continue;
                }
                let new_object_frame = FrameObjectData {
                    position: object.position,
                    rotation: object.rotation,
                    scale: object.scale,
                };
                if *self.registered_meshes.get(&object.mesh_path).unwrap() == true {
                    ipi.push(IPooledInstance {
                        obj: RenderObject {
                            previous_frame: ofd,
                            current_frame: new_object_frame.clone(),
                        },
                        mesh_path: "./cube".into(),
                    });
                }
                self.objects.push(object);
                self.customs.push(custom);
                self.object_frames.push(new_object_frame);
                self.object_infos.push(info);
            }
        }

        callbacks.exit(&mut self);
    }

    fn check_mesh_load_tasks(&mut self, to_mainframe: &EventLoopProxy<ToMainframe>) {
        for mesh_load_task in &mut self.mesh_load_tasks {
            let p = mesh_load_task.poll();
            if p.is_pending() {
                continue;
            }
            let res = p.take_ready();
            match res {
                Err(msg) => println!("{msg}"),
                Ok(mesh) => {
                    let _ = to_mainframe.send_event(ToMainframe::PoolMeshRequest(mesh));
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3};
    use winit::{event_loop::EventLoop, platform::wayland::EventLoopBuilderExtWayland};

    struct TestGameCallbacks;

    impl GameCallbacks for TestGameCallbacks {
        fn start(&mut self, _game: &mut GameSimulation) {}

        fn update(&mut self, _game: &mut GameSimulation, _dt: f32) {
            println!("Update");
        }

        fn exit(&mut self, _game: &mut GameSimulation) {}
    }

    #[test]
    fn test_game_simulation() {
        let view_frustum = ViewFrustum::new(90.0, 0.1, 1000.0);
        let camera = Camera::new(Vec3::new(0.0, 0.0, 0.0), Quat::IDENTITY, view_frustum);
        let physics = Physics {
            tick_rate: 13,
            _gravity: Vec3::new(0.0, -9.8, 0.0),
        };
        let simulation = GameSimulation::new(camera, physics);
        let (s, r) = mpsc::channel();

        let e = EventLoop::<ToMainframe>::with_user_event()
            .with_any_thread(true)
            .build()
            .unwrap();
        e.set_control_flow(winit::event_loop::ControlFlow::Poll);
        let proxy = e.create_proxy();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            s.send(ToGame::Start).unwrap();
            thread::sleep(Duration::from_secs(5));
            let _ = s.send(ToGame::Stop);
        });
        simulation.simulate(r, proxy, Box::new(TestGameCallbacks));
    }
}
