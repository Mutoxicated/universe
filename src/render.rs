use std::sync::Arc;

use crate::game::{self, Camera};
use glam::{Mat4, Quat, Vec3};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::default().to_cols_array_2d(),
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.view_proj = camera.wgpu_view_proj_matrix().to_cols_array_2d();
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[derive(Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub name: Arc<str>,
}

impl Mesh {
    pub fn empty() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            name: "".into(),
        }
    }
}

pub fn load_gltf_into_mesh(mesh_path: Arc<str>) -> Result<Mesh, Arc<str>> {
    let scenes = easy_gltf::load(mesh_path.to_string());
    if let Err(x) = scenes {
        return Err(format!("Failed to load mesh: {x}").into());
    }
    let scenes = scenes.unwrap();
    if scenes.is_empty() {
        return Err("There was no mesh to load bruh!".into());
    }

    let model = &scenes[0].models[0];
    let vertices = Vec::<Vertex>::new();
    let indices = Vec::<u16>::new();

    let model_vertices = model.vertices();
    let material = model.material();

    Ok(Mesh::empty())
}

/// Renderable object data for ONE frame
#[derive(Copy, Clone)]
pub struct FrameObjectData {
    pub position: Vec3,
    pub scale: Vec3,
    pub rotation: Quat,
}

pub struct RenderObject {
    pub previous_frame: FrameObjectData,
    pub current_frame: FrameObjectData,
}

pub struct ISingleInstance {
    pub obj: RenderObject,
    pub mesh: Mesh,
}

pub struct IPooledInstance {
    pub obj: RenderObject,
    pub mesh_path: Arc<str>,
}

pub struct RenderBatch {
    pub camera: game::Camera,
    pub isi: Arc<[ISingleInstance]>,
    pub ipi: Arc<[IPooledInstance]>,
}

impl RenderBatch {
    pub fn empty() -> Self {
        Self {
            camera: Camera::DEFAULT,
            isi: vec![].into(),
            ipi: vec![].into(),
        }
    }
}
