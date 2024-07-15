use crate::{camera::CameraUniform, Vertex};
use std::{rc::Rc, sync::Arc};

use wgpu::{include_wgsl, util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupLayout, BlendState, ColorTargetState, CommandEncoder, Device};

use crate::{camera::{VIEWPORT_HEIGHT, VIEWPORT_WIDTH}, texture::Texture, texture_atlas::DeferredTextureRegion};

use super::super::pipeline::Pipeline;

pub struct DeferredPass {
    sprites: DeferredSpriteBatch,
    deferred_texture_bind_group: BindGroup,
    sheet: Arc<Texture>,
    pipeline: Pipeline,
    pub albedo_buffer: Texture,
    pub normal_buffer: Texture,
}
impl DeferredPass {
    pub fn draw_sprite(
        &mut self,
        position: (f32, f32, f32),
        image: DeferredTextureRegion,
    ) {
        if self.sheet == image.texture {
            self.sprites.sprites.push((position, image));
        } else {
            panic!("sprite had wrong sheet!")
        }
    }
    pub fn new(device: &Device, sheet: Arc<Texture>) -> DeferredPass {
        let deferred_texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                // This should match the filterable field of the
                // corresponding Texture entry above.
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            ],
            label: Some("texture_bind_group_layout"),
        });
        
        let deferred_texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("deferred_texture_bind_group"),
            layout: &deferred_texture_bind_group_layout,
            entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&sheet.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sheet.sampler),
            },
            ],
        });
        let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let pipeline = Pipeline::new::<DeferredVertex>(&device, include_wgsl!("../../deferred.wgsl"), &[&deferred_texture_bind_group_layout, &camera_bind_group_layout], &[Some(ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        }),
        Some(ColorTargetState {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
        })
        ], "DeferredPass");
        
        
        let albedo_buffer = Texture::create_texture(&device, Some("albedo_buffer"), (VIEWPORT_WIDTH as u32, VIEWPORT_HEIGHT as u32), wgpu::TextureFormat::Rgba8Unorm).unwrap();
        let normal_buffer = Texture::create_texture(&device, Some("normal_buffer"), (VIEWPORT_WIDTH as u32, VIEWPORT_HEIGHT as u32), wgpu::TextureFormat::Rgba8Unorm).unwrap();
        DeferredPass {
            sprites: DeferredSpriteBatch::new(),
            deferred_texture_bind_group,
            sheet,
            pipeline, 
            albedo_buffer,
            normal_buffer,
        }
    }
    
    pub fn draw(&mut self, device: &Device, encoder: &mut CommandEncoder, camera_bind_group: &BindGroup) {
        
        let (vertices, indices) = self.sprites.gen_vecs();
        let vertex_buffer = device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices.as_slice()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices.as_slice()),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        let mut deferred_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("G-Buffer Pass"),
            color_attachments: &[
            Some(wgpu::RenderPassColorAttachment {
                view: &self.albedo_buffer.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.05,
                        g: 0.05,
                        b: 0.05,
                        a: 1.,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: &self.normal_buffer.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.5,
                        g: 0.5,
                        b: 1.0,
                        a: 1.,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            }),
            ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        deferred_pass.set_pipeline(&self.pipeline.pipeline);
        deferred_pass.set_bind_group(0, &self.deferred_texture_bind_group, &[]);
        deferred_pass.set_bind_group(1, camera_bind_group, &[]);
        deferred_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        deferred_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        deferred_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        self.sprites.sprites.clear();
    }
    
}

struct DeferredSpriteBatch {
    sprites: Vec<((f32, f32, f32), DeferredTextureRegion)>,
}
impl DeferredSpriteBatch {
    fn new() -> DeferredSpriteBatch {
        DeferredSpriteBatch {
            sprites: Vec::new(),
        }
    }
    fn gen_vecs(&self) -> (Vec<DeferredVertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        for (index, (position, region)) in self.sprites.iter().enumerate() {
            vertices.push(DeferredVertex {
                position: [position.0, position.1, position.2],
                albedo_coords: [
                region.albedo.x as f32 / region.texture.width() as f32,
                (region.albedo.y + region.albedo.height) as f32
                / region.texture.height() as f32,
                ],
                normal_coords: [
                region.normal.x as f32 / region.texture.width() as f32,
                (region.normal.y + region.normal.height) as f32
                / region.texture.height() as f32,
                ],
            });
            vertices.push(DeferredVertex {
                position: [
                position.0 + region.albedo.width as f32,
                position.1,
                position.2,
                ],
                albedo_coords: [
                (region.albedo.x + region.albedo.width) as f32 / region.texture.width() as f32,
                (region.albedo.y + region.albedo.height) as f32
                / region.texture.height() as f32,
                ],
                normal_coords: [
                (region.normal.x + region.normal.width) as f32 / region.texture.width() as f32,
                (region.normal.y + region.normal.height) as f32
                / region.texture.height() as f32,
                ],
            });
            vertices.push(DeferredVertex {
                position: [
                position.0 + region.albedo.width as f32,
                position.1 + region.albedo.height as f32,
                position.2,
                ],
                albedo_coords: [
                (region.albedo.x + region.albedo.width) as f32 / region.texture.width() as f32,
                region.albedo.y as f32 / region.texture.height() as f32,
                ],
                normal_coords: [
                (region.normal.x + region.normal.width) as f32 / region.texture.width() as f32,
                region.normal.y as f32 / region.texture.height() as f32,
                ],
            });
            vertices.push(DeferredVertex {
                position: [
                position.0,
                position.1 + region.albedo.height as f32,
                position.2,
                ],
                albedo_coords: [
                region.albedo.x as f32 / region.texture.width() as f32,
                region.albedo.y as f32 / region.texture.height() as f32,
                ],
                normal_coords: [
                region.normal.x as f32 / region.texture.width() as f32,
                region.normal.y as f32 / region.texture.height() as f32,
                ],
            });
            indices.extend_from_slice(&[
                (4 * index).try_into().unwrap(),
                (1 + 4 * index).try_into().unwrap(),
                (2 + 4 * index).try_into().unwrap(),
                (2 + 4 * index).try_into().unwrap(),
                (3 + 4 * index).try_into().unwrap(),
                (4 * index).try_into().unwrap(),
            ]);
        }
        (vertices, indices)
    }
}
    
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Vertex)]
struct DeferredVertex {
    position: [f32; 3],
    albedo_coords: [f32; 2],
    normal_coords: [f32; 2],
}