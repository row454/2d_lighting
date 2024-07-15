use std::{collections::HashMap, iter};
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    assets::TextureAtlasStorage, camera::{Camera, CameraUniform, VIEWPORT_HEIGHT, VIEWPORT_WIDTH}, gfx::{context::GraphicsContext, pass::{deferred_pass::DeferredPass, lighting_pass::{self, DynamicLight, LightingPass}, present_pass::PresentPass}}, texture::TextureCreator, texture_atlas::{DeferredTextureRegion, TextureRegion}
};

pub struct RendererState {
    context: GraphicsContext,
    pub camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    viewport_matrix: [[f32; 4]; 4],

    
    deferred_pass: DeferredPass,
    lighting_pass: LightingPass,
    present_pass: PresentPass,
}

impl RendererState {
    pub async fn new(window: &Window, textures: &mut TextureAtlasStorage) -> RendererState {
        let size = window.inner_size();

        let context = GraphicsContext::new(window).await;

        let camera = Camera {
            pos: (0.0, 0.0, 10.0).into(),
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = context.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
        context.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = context.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        let max_scale =
            (size.width as f32 / VIEWPORT_WIDTH).min(size.height as f32 / VIEWPORT_HEIGHT);
        let viewport_matrix = cgmath::Matrix4::from_nonuniform_scale(
            max_scale / (size.width as f32 / VIEWPORT_WIDTH),
            max_scale / (size.height as f32 / VIEWPORT_HEIGHT),
            1.0,
        )
        .into();
        let sheet = textures.load("textures", &TextureCreator {
            device: &context.device,
            queue: &context.queue
        }).unwrap().image.clone();
        let deferred_pass = DeferredPass::new(&context.device, sheet);
        let lighting_pass = LightingPass::new(&context.device);
        let present_pass = PresentPass::new(&context.device, context.config.format);
        Self {
            context,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            viewport_matrix,
            deferred_pass,
            lighting_pass,
            present_pass
        }
    }

    pub fn texture_creator(&self) -> TextureCreator {
        TextureCreator {
            device: &self.context.device,
            queue: &self.context.queue,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.context.resize(width, height);
            let max_scale = (width as f32 / VIEWPORT_WIDTH)
                .min(height as f32 / VIEWPORT_HEIGHT);
            self.viewport_matrix = cgmath::Matrix4::from_nonuniform_scale(
                max_scale / (width as f32 / VIEWPORT_WIDTH),
                max_scale / (height as f32 / VIEWPORT_HEIGHT),
                1.0,
            )
            .into();
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self.context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        
        self.deferred_pass.draw(&self.context.device, &mut encoder, &self.camera_bind_group);
        self.lighting_pass.draw(&self.context.device, &mut encoder, &self.camera_bind_group, &self.deferred_pass.albedo_buffer, &self.deferred_pass.normal_buffer);
        self.present_pass.present(&self.context.device, encoder, &self.context.queue, &self.lighting_pass.output, self.viewport_matrix, &self.context.surface);

        Ok(())
    }
    pub fn draw_light(&mut self, light: DynamicLight) {
        self.lighting_pass.draw_light(light)
    }
    pub fn draw_deferred_sprite(
        &mut self,
        position: (f32, f32, f32),
        image: DeferredTextureRegion,
    ) {
        self.deferred_pass.draw_sprite(position, image)
    }
}
