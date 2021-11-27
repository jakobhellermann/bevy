use bevy_app::Plugin;
use bevy_asset::{Assets, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_render2::{
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, CachedPipelineId, ColorTargetState,
        ColorWrites, FragmentState, FrontFace, LoadOp, MultisampleState, Operations, PolygonMode,
        PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineCache, RenderPipelineDescriptor, SamplerDescriptor, Shader, ShaderStages,
        SpecializedPipeline, SpecializedPipelines, TextureFormat, TextureSampleType,
        TextureViewDimension, VertexState,
    },
    renderer::{RenderContext, RenderDevice},
    texture::BevyDefault,
    view::{ExtractedView, ViewTarget},
    RenderApp, RenderStage,
};

pub(crate) struct UpscalePassPlugin;
impl Plugin for UpscalePassPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        shaders.set_untracked(
            UPSCALE_SHADER_HANDLE,
            Shader::from_wgsl(include_str!("upscale_pass.wgsl")),
        );

        let render_app = app.sub_app(RenderApp);

        render_app
            .init_resource::<UpscalePassPipeline>()
            .init_resource::<SpecializedPipelines<UpscalePassPipeline>>()
            .add_system_to_stage(RenderStage::Queue, queue_upscale_pipeline)
            .add_system_to_stage(RenderStage::Queue, queue_view_bind_groups);
    }
}

pub struct UpscalePassNode {
    query: QueryState<(&'static ViewTarget, &'static ViewBindGroup), With<ExtractedView>>,
}
impl UpscalePassNode {
    pub fn new(render_world: &mut World) -> Self {
        UpscalePassNode {
            query: QueryState::new(render_world),
        }
    }
}

impl Node for UpscalePassNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.get_resource::<CachedUpscalePassPipeline>().unwrap().0;
        let pipeline_cache = world.get_resource::<RenderPipelineCache>().unwrap();

        for (view, bind_group) in self.query.iter_manual(world) {
            let pass_descriptor = RenderPassDescriptor {
                label: Some("upscale_pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: &view.upscaled_texture,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            };

            let mut render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);

            let pipeline = pipeline_cache.get(pipeline).unwrap();

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &bind_group.0, &[]);
            render_pass.draw(0..6, 0..1);
        }
        Ok(())
    }
}

#[derive(Component)]
struct ViewBindGroup(BindGroup);

fn queue_view_bind_groups(
    mut commands: Commands,
    query: Query<(Entity, &ViewTarget)>,
    pipeline: Res<UpscalePassPipeline>,
    render_device: Res<RenderDevice>,
) {
    for (entity, target) in query.iter() {
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            ..Default::default()
        });
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&target.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        commands.entity(entity).insert(ViewBindGroup(bind_group));
    }
}

const UPSCALE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 18122257609416099947);

struct CachedUpscalePassPipeline(CachedPipelineId);

fn queue_upscale_pipeline(
    mut commands: Commands,
    pipeline: Res<UpscalePassPipeline>,
    mut pipelines: ResMut<SpecializedPipelines<UpscalePassPipeline>>,
    mut cache: ResMut<RenderPipelineCache>,
) {
    let id = pipelines.specialize(&mut cache, &*pipeline, ());
    commands.insert_resource(CachedUpscalePassPipeline(id));
}

pub(crate) struct UpscalePassPipeline {
    bind_group_layout: BindGroupLayout,
}

impl FromWorld for UpscalePassPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.get_resource::<RenderDevice>().unwrap();
        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(""),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler {
                            filtering: true,
                            comparison: false,
                        },
                        count: None,
                    },
                ],
            });

        UpscalePassPipeline { bind_group_layout }
    }
}

impl SpecializedPipeline for UpscalePassPipeline {
    type Key = ();

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("upscale_pipeline".into()),
            layout: Some(vec![self.bind_group_layout.clone()]),
            vertex: VertexState {
                shader: UPSCALE_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "vs_main".into(),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: UPSCALE_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}
