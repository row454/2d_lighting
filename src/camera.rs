use std::cmp::max;

use cgmath::EuclideanSpace;
use winit::dpi::PhysicalSize;

pub struct Camera {
    pub pos: cgmath::Point3<f32>,
    pub angle: cgmath::Rad<f32>,
}
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, window_size: PhysicalSize<u32>) {
        self.view_proj = camera.build_view_projection_matrix(window_size).into();
        println!("{:?}", self.view_proj);
    }
}


#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

const VIEWPORT_WIDTH: f32 = 320.0;
const VIEWPORT_HEIGHT: f32 = 180.0;
impl Camera {
    fn build_view_projection_matrix(&self, window_size: PhysicalSize<u32>) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::from_translation((-self.pos.x, -self.pos.y, 0.0).into());
        // let rot = cgmath::Matrix4::from_angle_x(-self.angle);
        // 2.
        let proj = cgmath::ortho(-160.0, 160.0, -90.0, 90.0, -500.0, 500.0);
        
        let max_scale = (window_size.width as f32 / VIEWPORT_WIDTH).min(window_size.height as f32 /VIEWPORT_HEIGHT);
        let letterbox = cgmath::Matrix4::from_nonuniform_scale(max_scale/(window_size.width as f32 / VIEWPORT_WIDTH), max_scale/(window_size.height as f32 /VIEWPORT_HEIGHT), 1.0);
        // 3.
        letterbox * OPENGL_TO_WGPU_MATRIX * proj * view
    }
}
