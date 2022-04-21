use std::{borrow::Cow, collections::VecDeque, num::NonZeroU32};

use bevy_ecs::prelude::World;
use bevy_utils::HashMap;
use wgpu::{ShaderStages, TextureFormat, TextureUsages};

use crate::{render_graph, render_resource::Texture, renderer::RenderDevice};

#[derive(Clone, Copy)]
pub enum MipmapGenerationMode {
    Render,
}

impl MipmapGenerationMode {
    pub fn preferred(_device: &RenderDevice) -> Self {
        MipmapGenerationMode::Render
    }

    pub fn required_texture_usages(&self) -> TextureUsages {
        match self {
            MipmapGenerationMode::Render => TextureUsages::RENDER_ATTACHMENT,
        }
    }
}

struct MipmapQueueRequest {
    mipmap_mode: MipmapGenerationMode,
    texture: Texture,
    mip_count: u32,
    texture_format: TextureFormat,
}

#[derive(Default)]
pub struct MipmapQueue {
    queue: VecDeque<MipmapQueueRequest>,
}

impl MipmapQueue {
    pub fn enqueue(
        &mut self,
        mipmap_mode: MipmapGenerationMode,
        texture: Texture,
        mip_count: u32,
        texture_format: TextureFormat,
    ) {
        self.queue.push_back(MipmapQueueRequest {
            mipmap_mode,
            texture,
            mip_count,
            texture_format,
        })
    }
}

pub struct MipmapGenerationNode {
    queue: Vec<MipmapQueueRequest>,
    shader: wgpu::ShaderModule,
    pipelines: HashMap<TextureFormat, wgpu::RenderPipeline>,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl render_graph::Node for MipmapGenerationNode {
    fn update(&mut self, world: &mut World) {
        let world = world.cell();
        let mut queue = world.get_resource_mut::<MipmapQueue>().unwrap();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        self.queue.clear();
        for request in queue.queue.drain(..) {
            self.init_pipeline(&render_device, request.texture_format);
            self.queue.push(request);
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut crate::renderer::RenderContext,
        _world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        for request in &self.queue {
            bevy_utils::tracing::debug!(
                "generating {} mipmaps for {:?}",
                request.mip_count,
                request.texture.id()
            );

            match request.mipmap_mode {
                MipmapGenerationMode::Render => {
                    self.generate_mipmaps_render(
                        &mut render_context.command_encoder,
                        render_context.render_device.wgpu_device(),
                        &request.texture,
                        request.mip_count,
                        request.texture_format,
                    );
                }
            }
        }

        Ok(())
    }
}

impl MipmapGenerationNode {
    pub fn new(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>().wgpu_device();

        let shader = render_device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("mip blit shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("blit.wgsl"))),
        });

        let bind_group_layout =
            render_device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("mip bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2, // TODO
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let sampler = render_device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mip sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        MipmapGenerationNode {
            queue: Vec::new(),
            shader,
            pipelines: HashMap::default(),
            bind_group_layout,
            sampler,
        }
    }

    fn init_pipeline(&mut self, render_device: &RenderDevice, texture_format: TextureFormat) {
        self.pipelines.entry(texture_format).or_insert_with(|| {
            render_device
                .wgpu_device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("blit"),
                    layout: None,
                    vertex: wgpu::VertexState {
                        module: &self.shader,
                        entry_point: "vs_main",
                        buffers: &[],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &self.shader,
                        entry_point: "fs_main",
                        targets: &[texture_format.into()],
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                })
        });
    }

    fn generate_mipmaps_render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        texture: &wgpu::Texture,
        mip_count: u32,
        texture_format: TextureFormat,
    ) {
        let views = (0..mip_count)
            .map(|mip| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    label: Some("mip texture view descriptor"),
                    format: None,
                    dimension: None,
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: mip,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: 0,
                    array_layer_count: None,
                })
            })
            .collect::<Vec<_>>();

        for target_mip in 1..mip_count as usize {
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&views[target_mip - 1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: Some("mip bind group"),
            });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mipmap generation rpass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &views[target_mip],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            // this will be populated in `Node::update`
            let pipeline = &self.pipelines[&texture_format];

            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }
    }
}
