use std::{iter, sync::Arc};

use wgpu::{include_wgsl, util::DeviceExt, BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, Buffer, Color, ColorTargetState, ColorWrites, CommandEncoder, Device, Queue, Surface, TextureFormat, TextureView};

use crate::{gfx::pipeline::Pipeline, texture::Texture, Vertex};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Vertex)]

struct PresentVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}
const VERTICES: &[PresentVertex] = &[
PresentVertex {
    position: [1., 1., 0.],
    tex_coords: [1., 0.],
},
PresentVertex {
    position: [-1., 1., 0.],
    tex_coords: [0., 0.],
},
PresentVertex {
    position: [-1., -1., 0.],
    tex_coords: [0., 1.],
},
PresentVertex {
    position: [1., -1., 0.],
    tex_coords: [1., 1.],
},
];
const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];
pub struct PresentPass {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    viewport_bind_group_layout: BindGroupLayout,
    texture_bind_group_layout: BindGroupLayout,
    pipeline: Pipeline,
}

impl PresentPass {
    pub fn new(device: &Device, format: TextureFormat) -> PresentPass {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        let viewport_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        
        let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
        let pipeline = Pipeline::new::<PresentVertex>(device, include_wgsl!("../../present.wgsl"), &[&texture_bind_group_layout, &viewport_bind_group_layout], &[Some(ColorTargetState {
            format,
            blend: None,
            write_mask: ColorWrites::ALL,
        }

        )], "PresentPass");
        PresentPass {
            vertex_buffer,
            index_buffer,
            viewport_bind_group_layout,
            texture_bind_group_layout,
            pipeline
        }
    }
    pub fn present(&self, device: &Device, mut encoder: CommandEncoder, queue: &Queue, to_present: &Texture, viewport_matrix: [[f32; 4]; 4], surface: &Surface) {
        let output_texture = surface.get_current_texture().unwrap();
        let output_view = output_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let viewport_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport Buffer"),
            contents: bytemuck::cast_slice(&[viewport_matrix]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
                
        let viewport_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.viewport_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_buffer.as_entire_binding(),
            }],
            label: Some("viewport_bind_group"),
        });
        let texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("texture_bind_group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&to_present.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&to_present.sampler),
            },
            ],
        });

        let mut present_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Present Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        present_pass.set_pipeline(&self.pipeline.pipeline);
        present_pass.set_bind_group(0, &texture_bind_group, &[]);
        present_pass.set_bind_group(1, &viewport_bind_group, &[]);
        present_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        present_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        present_pass.draw_indexed(0..6, 0, 0..1);

        std::mem::drop(present_pass);

        queue.submit(iter::once(encoder.finish()));
        output_texture.present();
    }
}



/*
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
*/