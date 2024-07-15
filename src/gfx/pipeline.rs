use wgpu::{BindGroupLayout, ColorTargetState, Device, ShaderModuleDescriptor};

use crate::Vertex;

pub struct Pipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
    // use wgpu::include_wgsl!("shader.wgsl")
    pub fn new<V: Vertex>(device: &Device, shader: ShaderModuleDescriptor, bind_group_layouts: &[&BindGroupLayout], targets: &[Option<ColorTargetState>], name: &'static str) -> Pipeline {
        let shader = device.create_shader_module(shader);
        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(format!("{} Pipeline Layout", name).as_str()),
                bind_group_layouts,
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(format!("{} Pipeline", name).as_str()),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[V::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets,
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
        Pipeline {
            pipeline
        }
    }
}