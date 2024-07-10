use std::{collections::HashMap, iter, sync::Arc};
use wgpu::{util::DeviceExt, BindGroupDescriptor, BindGroupLayout};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    camera::{Camera, CameraUniform},
    texture::{Texture, TextureCreator},
    texture_atlas::{DeferredTextureRegion, TextureRegion},
    DeferredVertex, Vertex,
};

pub struct RendererState<'a> {
    surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: PhysicalSize<u32>,
    window: &'a Window,
    pub clear_color: wgpu::Color,
    vertex_pipeline: wgpu::RenderPipeline,
    pub camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    sprite_batches: HashMap<Arc<Texture>, SpriteBatch>,
    deferred_sprite_batches: HashMap<Arc<Texture>, DeferredSpriteBatch>,
    deferred_texture_bind_group_layout: BindGroupLayout,
    viewport_matrix: [[f32; 4]; 4],
    g_buffer_pipeline: wgpu::RenderPipeline,
}
struct SpriteBatch {
    sprites: Vec<((f32, f32, f32), TextureRegion)>,
}
impl SpriteBatch {
    fn new() -> SpriteBatch {
        SpriteBatch {
            sprites: Vec::new(),
        }
    }
    fn gen_vecs(&self) -> (Vec<Vertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        for (index, (position, region)) in self.sprites.iter().enumerate() {
            vertices.push(Vertex {
                position: [position.0, position.1, position.2],
                tex_coords: [
                    region.src.x as f32 / region.texture.width() as f32,
                    (region.src.y + region.src.height) as f32 / region.texture.height() as f32,
                ],
            });
            vertices.push(Vertex {
                position: [position.0 + region.src.width as f32, position.1, position.2],
                tex_coords: [
                    (region.src.x + region.src.width) as f32 / region.texture.width() as f32,
                    (region.src.y + region.src.height) as f32 / region.texture.height() as f32,
                ],
            });
            vertices.push(Vertex {
                position: [
                    position.0 + region.src.width as f32,
                    position.1 + region.src.height as f32,
                    position.2,
                ],
                tex_coords: [
                    (region.src.x + region.src.width) as f32 / region.texture.width() as f32,
                    region.src.y as f32 / region.texture.height() as f32,
                ],
            });
            vertices.push(Vertex {
                position: [
                    position.0,
                    position.1 + region.src.height as f32,
                    position.2,
                ],
                tex_coords: [
                    region.src.x as f32 / region.texture.width() as f32,
                    region.src.y as f32 / region.texture.height() as f32,
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
struct Lights {
    lights: Vec<((f32, f32, f32), Light)>,
}
impl Lights {
    fn new() -> Lights {
        Lights { lights: Vec::new() }
    }
    fn gen_vecs(&self) -> (Vec<LightVertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        for (index, (position, Light { radius, color })) in self.lights.iter().enumerate() {
            let color = [color.0, color.1, color.2];
            vertices.push(LightVertex {
                position: [position.0 - radius, position.1 - radius, position.2],
                color,
            });
            vertices.push(LightVertex {
                position: [position.0 + radius, position.1 - radius, position.2],
                color,
            });
            vertices.push(LightVertex {
                position: [position.0 + radius, position.1 + radius, position.2],
                color,
            });
            vertices.push(LightVertex {
                position: [position.0 - radius, position.1 + radius, position.2],
                color,
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

struct Light {
    color: (f32, f32, f32),
    radius: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightVertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl LightVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
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

impl<'a> RendererState<'a> {
    pub async fn new(window: &'a Window) -> RendererState<'a> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,

            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout =
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
        let camera = Camera {
            pos: (0.0, 0.0, 10.0).into(),
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let deferred_shader = device.create_shader_module(wgpu::include_wgsl!("deferred.wgsl"));

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

        let g_buffer_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &deferred_texture_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let g_buffer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("g_buffer Pipeline"),
            layout: Some(&g_buffer_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &deferred_shader,
                entry_point: "vs_deferred",
                buffers: &[DeferredVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &deferred_shader,
                entry_point: "fs_deferred",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        let max_scale =
            (size.width as f32 / VIEWPORT_WIDTH).min(size.height as f32 / VIEWPORT_HEIGHT);
        let viewport_matrix = cgmath::Matrix4::from_nonuniform_scale(
            max_scale / (size.width as f32 / VIEWPORT_WIDTH),
            max_scale / (size.height as f32 / VIEWPORT_HEIGHT),
            1.0,
        )
        .into();
        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            clear_color: wgpu::Color {
                r: 0.1,
                g: 0.2,
                b: 0.3,
                a: 1.0,
            },
            vertex_pipeline: render_pipeline,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            sprite_batches: HashMap::new(),
            viewport_matrix,
            g_buffer_pipeline,
            deferred_sprite_batches: HashMap::new(),
            deferred_texture_bind_group_layout,
        }
    }

    pub fn texture_creator(&self) -> TextureCreator {
        TextureCreator {
            device: &self.device,
            queue: &self.queue,
        }
    }
    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            let max_scale = (self.size.width as f32 / VIEWPORT_WIDTH)
                .min(self.size.height as f32 / VIEWPORT_HEIGHT);
            self.viewport_matrix = cgmath::Matrix4::from_nonuniform_scale(
                max_scale / (self.size.width as f32 / VIEWPORT_WIDTH),
                max_scale / (self.size.height as f32 / VIEWPORT_HEIGHT),
                1.0,
            )
            .into();
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        let normal_buffer = Texture::create_texture(
            &self.device,
            Some("normal_buffer"),
            (VIEWPORT_WIDTH as u32, VIEWPORT_HEIGHT as u32),
            wgpu::TextureFormat::Bgra8Unorm,
        )
        .unwrap();
        let albedo_buffer = Texture::create_texture(
            &self.device,
            Some("color_buffer"),
            (VIEWPORT_WIDTH as u32, VIEWPORT_HEIGHT as u32),
            wgpu::TextureFormat::Bgra8Unorm,
        )
        .unwrap();
        let sprite_buffer = Texture::create_texture(
            &self.device,
            Some("color_buffer"),
            (VIEWPORT_WIDTH as u32, VIEWPORT_HEIGHT as u32),
            wgpu::TextureFormat::Bgra8UnormSrgb,
        )
        .unwrap();

        // {
        //     let _clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        //         label: Some("Clear Pass"),
        //         color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        //             view: &view,
        //             resolve_target: None,
        //             ops: wgpu::Operations {
        //                 load: wgpu::LoadOp::Clear(self.clear_color),
        //                 store: wgpu::StoreOp::Store
        //             }
        //         })],
        //         depth_stencil_attachment: None,
        //         occlusion_query_set: None,
        //         timestamp_writes: None
        //     });
        // }
        const TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR: wgpu::BindGroupLayoutDescriptor =
            wgpu::BindGroupLayoutDescriptor {
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
            };

        for (sheet, batch) in self.deferred_sprite_batches.iter() {
            let deferred_texture_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("deferred_texture_bind_group"),
                layout: &self.deferred_texture_bind_group_layout,
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
            let (vertices, indices) = batch.gen_vecs();
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices.as_slice()),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(indices.as_slice()),
                    usage: wgpu::BufferUsages::INDEX,
                });
            println!("{:?} {:?}", vertices, indices);
            let mut g_buffer_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("G-Buffer Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &albedo_buffer.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &normal_buffer.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
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

            g_buffer_pass.set_pipeline(&self.g_buffer_pipeline);
            g_buffer_pass.set_bind_group(0, &deferred_texture_bind_group, &[]);
            g_buffer_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            g_buffer_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            g_buffer_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            g_buffer_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }

        for (sheet, batch) in self.sprite_batches.iter() {
            let texture_bind_group_layout = self
                .device
                .create_bind_group_layout(&TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR);
            let texture_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("deferred_texture_bind_group"),
                layout: &texture_bind_group_layout,
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
            let (vertices, indices) = batch.gen_vecs();
            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(vertices.as_slice()),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(indices.as_slice()),
                    usage: wgpu::BufferUsages::INDEX,
                });
            println!("{:?} {:?}", vertices, indices);
            let mut sprite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite_buffer Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &sprite_buffer.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            sprite_pass.set_pipeline(&self.vertex_pipeline);
            sprite_pass.set_bind_group(0, &texture_bind_group, &[]);
            sprite_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            sprite_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            sprite_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            sprite_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);
        }
        self.deferred_sprite_batches.clear();
        self.sprite_batches.clear();

        {
            const VERTICES: &[Vertex] = &[
                Vertex {
                    position: [1., 1., 0.],
                    tex_coords: [1., 0.],
                },
                Vertex {
                    position: [-1., 1., 0.],
                    tex_coords: [0., 0.],
                },
                Vertex {
                    position: [-1., -1., 0.],
                    tex_coords: [0., 1.],
                },
                Vertex {
                    position: [1., -1., 0.],
                    tex_coords: [1., 1.],
                },
            ];
            const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

            let vertex_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(VERTICES),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let index_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(INDICES),
                    usage: wgpu::BufferUsages::INDEX,
                });

            let viewport_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("viewport Buffer"),
                        contents: bytemuck::cast_slice(&[self.viewport_matrix]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });
            let viewport_bind_group_layout =
                self.device
                    .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                        label: Some("viewport_bind_group_layout"),
                    });

            let viewport_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &viewport_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_buffer.as_entire_binding(),
                }],
                label: Some("viewport_bind_group"),
            });

            let texture_bind_group_layout = self
                .device
                .create_bind_group_layout(&TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR);

            let texture_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("texture_bind_group"),
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&albedo_buffer.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&albedo_buffer.sampler),
                    },
                ],
            });

            let mut present_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Present Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            present_pass.set_pipeline(&self.vertex_pipeline);
            present_pass.set_bind_group(0, &texture_bind_group, &[]);
            present_pass.set_bind_group(1, &viewport_bind_group, &[]);
            present_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            present_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            present_pass.draw_indexed(0..6, 0, 0..1);
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn draw_sprite(&mut self, position: (f32, f32, f32), image: TextureRegion) {
        if let Some(sprite_batch) = self.sprite_batches.get_mut(&image.texture) {
            sprite_batch.sprites.push((position, image));
        } else {
            let mut sprite_batch = SpriteBatch::new();
            sprite_batch.sprites.push((position, image.clone()));
            self.sprite_batches.insert(image.texture, sprite_batch);
        }
    }

    pub fn draw_deferred_sprite(
        &mut self,
        position: (f32, f32, f32),
        image: DeferredTextureRegion,
    ) {
        if let Some(sprite_batch) = self.deferred_sprite_batches.get_mut(&image.texture) {
            sprite_batch.sprites.push((position, image));
        } else {
            let mut sprite_batch = DeferredSpriteBatch::new();
            sprite_batch.sprites.push((position, image.clone()));
            self.deferred_sprite_batches
                .insert(image.texture, sprite_batch);
        }
    }

    pub fn draw_light(&mut self, position: (f32, f32, f32), light: Light) {}
}
