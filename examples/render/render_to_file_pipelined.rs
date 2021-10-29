use std::{fs::File, io::Write, num::NonZeroU32};

use bevy::{
    app::AppExit,
    ecs::prelude::*,
    math::Vec3,
    pbr2::{PbrBundle, PointLightBundle, StandardMaterial},
    prelude::{App, Assets, Handle, Plugin, Transform},
    render2::{
        camera::{
            Camera, CameraPlugin, CameraTarget, OrthographicCameraBundle, PerspectiveCameraBundle,
        },
        color::Color,
        mesh::{shape, Mesh},
        render_asset::RenderAssets,
        render_graph::{Node, RenderGraph},
        render_resource::{Buffer, Extent3d, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderDevice,
        texture::{Image, TextureFormatPixelInfo},
        view::Msaa,
        RenderApp, RenderStage,
    },
    PipelinedDefaultPlugins,
};
use wgpu::{BufferDescriptor, BufferUsages, ImageDataLayout};

const RENDER_TEXTURE_SIZE: (usize, usize) = (512, 200);

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(PipelinedDefaultPlugins)
        .add_plugin(RenderToTexturePlugin)
        .add_startup_system(setup)
        .add_system(exit_on_second_frame)
        .run();
}

#[derive(Clone)]
struct RenderTexture(Handle<Image>, BufferDimensions);

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
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
    let handle = images.add(image);
    commands.insert_resource(RenderTexture(
        handle.clone(),
        BufferDimensions::new(RENDER_TEXTURE_SIZE.0, RENDER_TEXTURE_SIZE.1),
    ));

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            name: Some(CameraPlugin::CAMERA_3D.to_string()),
            target: CameraTarget::Texture(handle),
            ..Default::default()
        },
        ..Default::default()
    });

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

struct RenderToTexturePlugin;
impl Plugin for RenderToTexturePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app(RenderApp);

        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        render_graph.add_node("readback_render_texture", ReadbackRenderTextureNode);
        render_graph
            .add_node_edge(
                bevy::core_pipeline::node::MAIN_PASS_DRIVER,
                "readback_render_texture",
            )
            .unwrap();

        let render_device = render_app.world.get_resource::<RenderDevice>().unwrap();
        let buffer_dimensions = BufferDimensions::new(RENDER_TEXTURE_SIZE.0, RENDER_TEXTURE_SIZE.1);
        let readback_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: (buffer_dimensions.padded_bytes_per_row * buffer_dimensions.height) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        render_app.insert_resource(ReadbackTextureBuffer(readback_buffer));

        render_app.add_system_to_stage(RenderStage::Extract, extract_render_texture);
    }
}

fn extract_render_texture(mut commands: Commands, render_texture: Res<RenderTexture>) {
    commands.insert_resource(render_texture.clone());
}

struct ReadbackTextureBuffer(Buffer);

struct ReadbackRenderTextureNode;
impl Node for ReadbackRenderTextureNode {
    fn run(
        &self,
        _: &mut bevy::render2::render_graph::RenderGraphContext,
        render_context: &mut bevy::render2::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render2::render_graph::NodeRunError> {
        let images = world.get_resource::<RenderAssets<Image>>().unwrap();
        let render_texture = world.get_resource::<RenderTexture>().unwrap();
        let readback_buffer = &world.get_resource::<ReadbackTextureBuffer>().unwrap().0;
        let buffer_dimensions = &render_texture.1;

        let gpu_image = images.get(&render_texture.0).unwrap();

        let size = Extent3d {
            width: buffer_dimensions.width as u32,
            height: buffer_dimensions.height as u32,
            depth_or_array_layers: 1,
        };
        render_context.command_encoder.copy_texture_to_buffer(
            gpu_image.texture.as_image_copy(),
            wgpu::ImageCopyBuffer {
                buffer: &readback_buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        NonZeroU32::new(buffer_dimensions.padded_bytes_per_row as u32).unwrap(),
                    ),
                    rows_per_image: None,
                },
            },
            size,
        );

        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let buffer_slice = readback_buffer.slice(..);
        render_device.map_buffer(&buffer_slice, wgpu::MapMode::Read);
        let padded_buffer = buffer_slice.get_mapped_range();

        use png_encode as png;
        let f = File::create("texture.png").unwrap();
        let mut png_encoder = png::Encoder::new(
            f,
            buffer_dimensions.width as u32,
            buffer_dimensions.height as u32,
        );
        png_encoder.set_depth(png::BitDepth::Eight);
        png_encoder.set_color(png::ColorType::RGBA);
        let mut png_writer = png_encoder
            .write_header()
            .unwrap()
            .into_stream_writer_with_size(buffer_dimensions.unpadded_bytes_per_row);

        // from the padded_buffer we write just the unpadded bytes into the image
        for chunk in padded_buffer.chunks(buffer_dimensions.padded_bytes_per_row) {
            let row = &chunk[..buffer_dimensions.unpadded_bytes_per_row];
            // PERF: this could probably be faster
            for pixel in row.chunks_exact(4) {
                let rgba = [pixel[2], pixel[1], pixel[0], pixel[3]];
                png_writer.write_all(&rgba).unwrap();
            }
        }
        png_writer.finish().unwrap();

        drop(padded_buffer);
        readback_buffer.unmap();

        Ok(())
    }
}

fn exit_on_second_frame(mut local: Local<bool>, mut app_exit: EventWriter<AppExit>) {
    if *local {
        app_exit.send(AppExit);
    } else {
        *local = true;
    }
}

#[derive(Clone, Debug)]
struct BufferDimensions {
    width: usize,
    height: usize,
    padded_bytes_per_row: usize,
    unpadded_bytes_per_row: usize,
}

impl BufferDimensions {
    fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        Self {
            width,
            height,
            padded_bytes_per_row,
            unpadded_bytes_per_row,
        }
    }
}
