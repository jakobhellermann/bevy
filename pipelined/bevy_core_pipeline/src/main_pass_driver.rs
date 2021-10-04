use crate::ViewDepthTexture;
use bevy_ecs::world::World;
use bevy_render2::{
    camera::{CameraPlugin, ExtractedCamera, ExtractedCameraNames},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotValue},
    renderer::RenderContext,
    view::ExtractedWindows,
};

pub struct MainPassDriverNode {
    #[cfg(feature = "smaa-antialiasing")]
    smaa_target: std::sync::Mutex<smaa::SmaaTarget>,
}

impl MainPassDriverNode {
    pub fn new(render_world: &mut World) -> Self {
        #[cfg(feature = "smaa-antialiasing")]
        let smaa_target = {
            use bevy_render2::texture::BevyDefault;
            let render_device = render_world
                .get_resource::<bevy_render2::renderer::RenderDevice>()
                .unwrap();
            let render_queue = render_world
                .get_resource::<bevy_render2::renderer::RenderQueue>()
                .unwrap();
            let swap_chain_format = bevy_render2::render_resource::TextureFormat::bevy_default();

            smaa::SmaaTarget::new(
                render_device.wgpu_device(),
                render_queue,
                1280,
                720,
                swap_chain_format,
                smaa::SmaaMode::Smaa1X,
                smaa::ShaderQuality::Ultra,
            )
        };

        MainPassDriverNode {
            #[cfg(feature = "smaa-antialiasing")]
            smaa_target: std::sync::Mutex::new(smaa_target),
        }
    }
}

impl Node for MainPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        let extracted_windows = world.get_resource::<ExtractedWindows>().unwrap();

        if let Some(camera_2d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_2D) {
            let extracted_camera = world.entity(*camera_2d).get::<ExtractedCamera>().unwrap();
            let extracted_window = extracted_windows.get(&extracted_camera.window_id).unwrap();
            let swap_chain_texture = extracted_window.swap_chain_frame.as_ref().unwrap().clone();
            graph.run_sub_graph(
                crate::draw_2d_graph::NAME,
                vec![
                    SlotValue::Entity(*camera_2d),
                    SlotValue::TextureView(swap_chain_texture),
                ],
            )?;
        }

        if let Some(camera_3d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_3D) {
            let extracted_camera = world.entity(*camera_3d).get::<ExtractedCamera>().unwrap();
            let depth_texture = world.entity(*camera_3d).get::<ViewDepthTexture>().unwrap();
            let extracted_window = extracted_windows.get(&extracted_camera.window_id).unwrap();
            let swap_chain_texture = extracted_window.swap_chain_frame.as_ref().unwrap().clone();

            #[cfg(feature = "smaa-antialiasing")]
            let render_queue = &**world
                .get_resource::<bevy_render2::renderer::RenderQueue>()
                .unwrap();
            #[cfg(feature = "smaa-antialiasing")]
            let render_device = _render_context.render_device.wgpu_device();

            #[cfg(feature = "smaa-antialiasing")]
            let mut smaa_target = self.smaa_target.lock().unwrap();
            #[cfg(feature = "smaa-antialiasing")]
            let smaa_frame =
                smaa_target.start_frame(render_device, render_queue, &swap_chain_texture);

            #[cfg(feature = "smaa-antialiasing")]
            let texture_view = smaa_frame.bevy_color_target().unwrap().clone();
            #[cfg(not(feature = "smaa-antialiasing"))]
            let texture_view = swap_chain_texture;

            graph.run_sub_graph(
                crate::draw_3d_graph::NAME,
                vec![
                    SlotValue::Entity(*camera_3d),
                    SlotValue::TextureView(texture_view),
                    SlotValue::TextureView(depth_texture.view.clone()),
                ],
            )?;
        }

        Ok(())
    }
}
