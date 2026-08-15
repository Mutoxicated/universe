use std::sync::Arc;

use glam::{Quat, Vec3};

use crate::game::GameSimulation;

pub struct Object {
    pub position: Vec3,
    pub scale: Vec3,
    /// if it's empty, then that signifies to the engine that it doesn't have a mesh
    pub mesh_path: Arc<str>,
    pub rotation: Quat,
}

pub struct ObjectInfo {
    pub is_alive: bool,
}

pub trait Custom: Send + Sync {
    fn start(&mut self, obj: &mut Object, life_index: usize, game: &mut GameSimulation);
    fn update(&mut self, obj: &mut Object, life_index: usize, game: &mut GameSimulation, dt: f32);
}
