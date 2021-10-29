use bevy::{
    core_pipeline::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::prelude::*,
    math::{Vec2, Vec3},
    pbr2::{PbrBundle, PointLightBundle, StandardMaterial},
    prelude::{App, Assets, Handle, Plugin, Transform},
    render2::{
        camera::{
            ActiveCameras, Camera, CameraTarget, ExtractedCameraNames, PerspectiveCameraBundle,
        },
        color::Color,
        mesh::{shape, Mesh},
        render_graph::{Node, RenderGraph, SlotValue},
        render_phase::RenderPhase,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        texture::{Image, TextureFormatPixelInfo},
        view::Msaa,
        RenderApp, RenderStage,
    },
    PipelinedDefaultPlugins,
};

const RENDER_TEXTURE_SIZE: (usize, usize) = (512, 200);

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(PipelinedDefaultPlugins)
        .add_plugin(RenderToTexturePlugin)
        .add_startup_system(setup)
        .add_system(swap_textures)
        .run();
}

struct Display;
struct Cam;

// a single texture can't be rendered to and drawn in the same frame,
// so double-buffer by swapping the camera target and display texture every frame.
fn swap_textures(
    mut camera: Query<&mut Camera, With<Cam>>,
    display: Query<&Handle<StandardMaterial>, With<Display>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut camera = camera.single_mut();
    let camera = match &mut camera.target {
        CameraTarget::Texture(render_texture) => render_texture,
        CameraTarget::Window(_) => unreachable!(),
    };
    let display = materials
        .get_mut(display.single())
        .unwrap()
        .base_color_texture
        .as_mut()
        .unwrap();

    std::mem::swap(camera, display);
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut active_cameras: ResMut<ActiveCameras>,
) {
    let mut image = Image::new(
        Extent3d {
            width: RENDER_TEXTURE_SIZE.0 as u32,
            height: RENDER_TEXTURE_SIZE.1 as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![
            1;
            RENDER_TEXTURE_SIZE.0 as usize
                * RENDER_TEXTURE_SIZE.1 as usize
                * TextureFormat::Bgra8UnormSrgb.pixel_size()
        ],
        TextureFormat::Bgra8UnormSrgb,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let image_material = images.add(image.clone());
    let image_render_target = images.add(image);

    let camera_position = Vec3::new(-2.0, 2.5, 5.0);

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 7.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Quad {
                size: Vec2::new(1.66, 1.0),
                flip: true,
            })),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(image_material),
                unlit: true,
                ..Default::default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, 0.0).looking_at(camera_position, Vec3::Y),
            ..Default::default()
        })
        .insert(Display);
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(camera_position).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                name: Some(RenderToTexturePlugin::CAMERA_RTT.to_string()),
                target: CameraTarget::Texture(image_render_target),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Cam);
    active_cameras.add(RenderToTexturePlugin::CAMERA_RTT);

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_translation(camera_position).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

struct RenderToTexturePlugin;
impl RenderToTexturePlugin {
    const CAMERA_RTT: &'static str = "camera_rtt";
}
impl Plugin for RenderToTexturePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app(RenderApp);
        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        render_graph.add_node("rtt_node", RenderToTextureDriverNode);
        render_graph
            .add_node_edge(bevy::core_pipeline::node::MAIN_PASS_DRIVER, "rtt_node")
            .unwrap();

        render_app.add_system_to_stage(RenderStage::Extract, extract_rtt_pipeline_camera_phases);
    }
}

fn extract_rtt_pipeline_camera_phases(mut commands: Commands, active_cameras: Res<ActiveCameras>) {
    if let Some(camera_rtt) = active_cameras.get(RenderToTexturePlugin::CAMERA_RTT) {
        if let Some(entity) = camera_rtt.entity {
            commands.get_or_spawn(entity).insert_bundle((
                RenderPhase::<Opaque3d>::default(),
                RenderPhase::<AlphaMask3d>::default(),
                RenderPhase::<Transparent3d>::default(),
            ));
        }
    }
}

struct RenderToTextureDriverNode;
impl Node for RenderToTextureDriverNode {
    fn run(
        &self,
        graph: &mut bevy::render2::render_graph::RenderGraphContext,
        _: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render2::render_graph::NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        if let Some(camera_rtt) = extracted_cameras
            .entities
            .get(RenderToTexturePlugin::CAMERA_RTT)
        {
            graph.run_sub_graph(
                bevy::core_pipeline::draw_3d_graph::NAME,
                vec![SlotValue::Entity(*camera_rtt)],
            )?;
        }
        Ok(())
    }
}
