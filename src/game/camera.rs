use glam::{Mat4, Quat, Vec3, camera::lh};

#[derive(Clone)]
pub struct ViewFrustum {
    aspect_ratio: f32,
    fov: f32,
    near: f32,
    far: f32,
}

impl ViewFrustum {
    /// Aspect ratio by default is 16:9
    pub fn new(fov: f32, near: f32, far: f32) -> ViewFrustum {
        Self {
            aspect_ratio: 16.0 / 9.0,
            fov,
            near,
            far,
        }
    }

    fn proj_matrix(&self) -> Mat4 {
        lh::proj::directx::perspective(self.fov, self.aspect_ratio, self.near, self.far)
    }
}

#[derive(Clone)]
pub struct Camera {
    position: Vec3,
    rotation: Quat,
    view_frustum: ViewFrustum,
}

impl Camera {
    pub const DEFAULT: Self = Self {
        position: Vec3::new(0.0, 0.0, 0.0),
        rotation: Quat::IDENTITY,
        view_frustum: ViewFrustum {
            aspect_ratio: 9.0 / 16.0,
            fov: 90.0,
            near: 0.1,
            far: 100.0,
        },
    };

    pub fn new(pos: Vec3, rot: Quat, view_frustum: ViewFrustum) -> Camera {
        Self {
            position: pos,
            rotation: rot,
            view_frustum,
        }
    }

    pub fn update_aspect_ratio(&mut self, ratio: f32) {
        self.view_frustum.aspect_ratio = ratio;
    }

    pub fn view_matrix(&self) -> Mat4 {
        lh::view::look_at_mat4(
            self.position,
            self.position + self.rotation.mul_vec3(Vec3::new(0.0, 0.0, 1.0)),
            Vec3::new(0.0, 1.0, 0.0),
        )
    }

    pub fn proj_matrix(&self) -> Mat4 {
        self.view_frustum.proj_matrix()
    }

    pub fn wgpu_view_proj_matrix(&self) -> Mat4 {
        return self.proj_matrix() * self.view_matrix();
    }
}
