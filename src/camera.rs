use winit::dpi::PhysicalSize;

pub struct Camera {
    pub pos: cgmath::Point3<f32>,
}
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    pub view_proj: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub viewport_dimensions: [f32; 4]
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
            view: cgmath::Matrix4::identity().into(),
            viewport_dimensions: [VIEWPORT_WIDTH, VIEWPORT_HEIGHT, MAX_DEPTH - MIN_DEPTH, 1.0],
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
        self.view = cgmath::Matrix4::from_translation((-camera.pos.x, -camera.pos.y, 0.0).into()).into();
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

pub const VIEWPORT_WIDTH: f32 = 320.0;
pub const VIEWPORT_HEIGHT: f32 = 180.0;
pub const MAX_DEPTH: f32 = 500.0;
pub const MIN_DEPTH: f32 = -500.0;
impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::from_translation((-self.pos.x, -self.pos.y, 0.0).into());
        // let rot = cgmath::Matrix4::from_angle_x(-self.angle);
        // 2.
        let proj = cgmath::ortho(
            -VIEWPORT_WIDTH / 2.0,
            VIEWPORT_WIDTH / 2.0,
            -VIEWPORT_HEIGHT / 2.0,
            VIEWPORT_HEIGHT / 2.0,
            MIN_DEPTH,
            MAX_DEPTH,
        );
        // 3.

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}
