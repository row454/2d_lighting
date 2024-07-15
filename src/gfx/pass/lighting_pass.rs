use wgpu::{include_wgsl, util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BlendComponent, BlendState, Buffer, ColorTargetState, ColorWrites, CommandEncoder, Device, RenderPassDescriptor};

use crate::{camera::{VIEWPORT_HEIGHT, VIEWPORT_WIDTH}, gfx::pipeline::Pipeline, texture::Texture, Vertex};

pub struct LightingPass {
    pipeline: Pipeline,
    pub output: Texture,
    lights: Lights,
    g_buffer_bind_group_layout: BindGroupLayout,

    global_light_pipeline: Pipeline,
    global_light_vertex_buffer: Buffer,
    global_light_index_buffer: Buffer,
}

impl LightingPass {

    pub fn new(device: &Device) -> LightingPass {
        let global_light_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let global_light_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let g_buffer_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                // This should match the filterable field of the
                // corresponding Texture entry above.
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            ],
            label: Some("texture_bind_group_layout"),
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

        let pipeline = Pipeline::new::<LightVertex>(device, include_wgsl!("../../light.wgsl"), &[
            &g_buffer_bind_group_layout,
            &camera_bind_group_layout,
        ], &[
            Some(ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALL,
            })
        ], "LightingPass");
        let global_light_pipeline = Pipeline::new::<GlobalLightVertex>(device, include_wgsl!("../../global_light.wgsl"), &[
            &g_buffer_bind_group_layout,
        ], &[
            Some(ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: wgpu::BlendFactor::One,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: BlendComponent::REPLACE,
                }),
                write_mask: ColorWrites::ALL,
            })
        ], "LightingPass global");

        LightingPass {
            pipeline,
            output: Texture::create_texture(device, Some("LightingPass output"), (VIEWPORT_WIDTH as u32, VIEWPORT_HEIGHT as u32), wgpu::TextureFormat::Rgba8UnormSrgb).unwrap(),
            lights: Lights::new(),
            g_buffer_bind_group_layout,
            global_light_vertex_buffer,
            global_light_index_buffer,
            global_light_pipeline,
        }
    }
    pub fn draw(&mut self, device: &Device, encoder: &mut CommandEncoder, camera_bind_group: &BindGroup, albedo_buffer: &Texture, normal_buffer: &Texture) {

        let (vertices, indices) = self.lights.gen_vecs();
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
        println!("{:?} {:?}", vertices, indices);
        let g_buffer_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("g_buffer_bind_group"),
            layout: &self.g_buffer_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&albedo_buffer.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&normal_buffer.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&albedo_buffer.sampler),
                },
            ],
        });
        let mut lighting_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("lighting_pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: &self.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                }),
                ],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        lighting_pass.set_pipeline(&self.pipeline.pipeline);
        lighting_pass.set_bind_group(0, &g_buffer_bind_group, &[]);
        lighting_pass.set_bind_group(1, camera_bind_group, &[]);
        lighting_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        lighting_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        lighting_pass.draw_indexed(0..indices.len() as u32, 0, 0..1);

        lighting_pass.set_pipeline(&self.global_light_pipeline.pipeline);
        lighting_pass.set_bind_group(0, &g_buffer_bind_group, &[]);
        lighting_pass.set_vertex_buffer(0, self.global_light_vertex_buffer.slice(..));
        lighting_pass.set_index_buffer(self.global_light_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        lighting_pass.draw_indexed(0..6 as u32, 0, 0..1);
        self.lights.lights.clear();
    }

    pub fn draw_light(&mut self, light: DynamicLight) {
        self.lights.lights.push(light);
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Vertex)]
struct GlobalLightVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
    color: [f32; 3],
}
const VERTICES: &[GlobalLightVertex] = &[
    GlobalLightVertex {
        position: [1., 1., 0.],
        tex_coords: [1., 0.],
        color: [0.1, 0.1, 0.1],
    },
    GlobalLightVertex {
        position: [-1., 1., 0.],
        tex_coords: [0., 0.],
        color: [0.1, 0.1, 0.1],
    },
    GlobalLightVertex {
        position: [-1., -1., 0.],
        tex_coords: [0., 1.],
        color: [0.1, 0.1, 0.1],
    },
    GlobalLightVertex {
        position: [1., -1., 0.],
        tex_coords: [1., 1.],
        color: [0.1, 0.1, 0.1],
    },
];
const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Vertex)]
struct LightVertex {
    position: [f32; 3],
    color: [f32; 3],
    center: [f32; 3],
    radius: f32,
}
struct Lights {
    lights: Vec<DynamicLight>
}
impl Lights {
    fn new() -> Lights {
        Lights {
            lights: Vec::new()
        }
    }
    fn gen_vecs(&self) -> (Vec<LightVertex>, Vec<u16>) {
        let mut vertices = Vec::new();
        let mut indices: Vec<u16> = Vec::new();
        for (index, DynamicLight {
            center, radius, color
        }) in self.lights.iter().enumerate() {
            vertices.push(LightVertex {
                position: [center.0 + radius, center.1 + radius, center.2],
                center: [center.0, center.1, center.2],
                color: (*color).into(),
                radius: *radius,
            });
            vertices.push(LightVertex {
                position: [center.0 - radius, center.1 + radius, center.2],
                center: [center.0, center.1, center.2],
                color: (*color).into(),
                radius: *radius,
            });
            vertices.push(LightVertex {
                position: [center.0 - radius, center.1 - radius, center.2],
                center: [center.0, center.1, center.2],
                color: (*color).into(),
                radius: *radius,
            });
            vertices.push(LightVertex {
                position: [center.0 + radius, center.1 - radius, center.2],
                center: [center.0, center.1, center.2],
                color: (*color).into(),
                radius: *radius,
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

#[derive(Clone, Copy)]
pub struct DynamicLight {
    pub center: (f32, f32, f32),
    pub radius: f32,
    pub color: Color
}


#[derive(Clone, Copy)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8
}
const FRAC_1_255_F64: f64 = 1./255.;
const FRAC_1_255_F32: f32 = 1./255.;
impl Into<[f32; 3]> for Color {
    fn into(self) -> [f32; 3] {
        [self.r as f32 * FRAC_1_255_F32, self.g as f32 * FRAC_1_255_F32, self.b as f32 * FRAC_1_255_F32]
    }
}
impl Into<wgpu::Color> for Color {
    fn into(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64 * FRAC_1_255_F64,
            g: self.g as f64 * FRAC_1_255_F64,
            b: self.b as f64 * FRAC_1_255_F64,
            a: 1.0
        }
    }
}
impl Color {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Color {
        Color {
            r,
            g,
            b
        }
    }
    pub fn from_hex(code: &'static str) -> Option<Color> {
        let code = code.strip_prefix("#").unwrap_or(code);
        if code.len() != 6 {
            return None
        };
        let r = u8::from_str_radix(&code[0..2], 16).ok()?;
        let g = u8::from_str_radix(&code[2..4], 16).ok()?;
        let b = u8::from_str_radix(&code[4..6], 16).ok()?;
        Some(Color {
                    r,
                    g,
                    b
                })
    }
}