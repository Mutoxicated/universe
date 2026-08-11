use std::sync::Arc;

use crate::game::{self, Camera};
use glam::{Mat4, Quat, Vec3, Vec4};

// pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
//     Vec::new(1.0, 0.0, 0.0, 0.0),
//     cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
//     cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
//     cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
// );

pub const OPENGL_TO_WGPU_MAT4: Mat4 = Mat4::from_cols(
    Vec4::new(1.0, 0.0, 0.0, 0.0),
    Vec4::new(0.0, 1.0, 0.0, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 0.0),
    Vec4::new(0.0, 0.0, 0.5, 1.0),
);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::default().into(),
        }
    }

    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            view_proj: camera.wgpu_view_proj_matrix().into(),
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.view_proj = camera.wgpu_view_proj_matrix().into();
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

/// Renderable object data for ONE frame
#[derive(Copy, Clone)]
pub struct FrameObjectData {
    pub position: Vec3,
    pub rotation: Quat,
}

pub enum RenderObject {
    SingleInstance {
        previous_frame: FrameObjectData,
        current_frame: FrameObjectData,
        mesh: Mesh,
    },
    PooledInstance {
        previous_frame: FrameObjectData,
        current_frame: FrameObjectData,
        mesh_name: Arc<str>,
    },
}

pub struct RenderBatch {
    pub camera: game::Camera,
    pub objects: Arc<[RenderObject]>,
}

impl RenderBatch {
    pub fn empty() -> Self {
        Self {
            camera: Camera::DEFAULT,
            objects: vec![].into(),
        }
    }
}
