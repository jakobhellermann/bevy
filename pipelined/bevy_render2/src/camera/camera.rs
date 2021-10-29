use crate::{camera::CameraProjection, texture::Image};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EventReader,
    prelude::{DetectChanges, QueryState},
    query::Added,
    reflect::ReflectComponent,
    system::{QuerySet, Res},
};
use bevy_math::{Mat4, Vec2, Vec3};
use bevy_reflect::{Reflect, ReflectDeserialize};
use bevy_transform::components::GlobalTransform;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
pub struct Camera {
    pub projection_matrix: Mat4,
    pub name: Option<String>,
    #[reflect(ignore)]
    pub depth_calculation: DepthCalculation,
    #[reflect(ignore)]
    pub target: CameraTarget,
}

#[derive(Debug, Clone)]
pub enum CameraTarget {
    Window(WindowId),
    Texture(Handle<Image>),
}
impl Default for CameraTarget {
    fn default() -> Self {
        CameraTarget::Window(WindowId::primary())
    }
}
impl CameraTarget {
    pub(crate) fn physical_size(
        &self,
        windows: &Windows,
        images: &Assets<Image>,
    ) -> Option<(u32, u32)> {
        match self {
            CameraTarget::Window(window_id) => {
                let window = windows.get(*window_id)?;
                Some((window.physical_width(), window.physical_height()))
            }
            CameraTarget::Texture(handle) => {
                let image = images.get(handle)?;
                Some((
                    image.texture_descriptor.size.width,
                    image.texture_descriptor.size.height,
                ))
            }
        }
    }
    pub(crate) fn size(&self, windows: &Windows, images: &Assets<Image>) -> Option<(f32, f32)> {
        match self {
            CameraTarget::Window(window_id) => {
                let window = windows.get(*window_id)?;
                Some((window.width(), window.height()))
            }
            CameraTarget::Texture(handle) => {
                let image = images.get(handle)?;
                Some((
                    image.texture_descriptor.size.width as f32,
                    image.texture_descriptor.size.height as f32,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum DepthCalculation {
    /// Pythagorean distance; works everywhere, more expensive to compute.
    Distance,
    /// Optimization for 2D; assuming the camera points towards -Z.
    ZDifference,
}

impl Default for DepthCalculation {
    fn default() -> Self {
        DepthCalculation::Distance
    }
}

impl Camera {
    /// Given a position in world space, use the camera to compute the screen space coordinates.
    pub fn world_to_screen(
        &self,
        windows: &Windows,
        images: &Assets<Image>,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Option<Vec2> {
        let size = Vec2::from(self.target.size(windows, images)?);
        // Build a transform to convert from world to NDC using camera data
        let world_to_ndc: Mat4 =
            self.projection_matrix * camera_transform.compute_matrix().inverse();
        let ndc_space_coords: Vec3 = world_to_ndc.project_point3(world_position);
        // NDC z-values outside of 0 < z < 1 are behind the camera and are thus not in screen space
        if ndc_space_coords.z < 0.0 || ndc_space_coords.z > 1.0 {
            return None;
        }
        // Once in NDC space, we can discard the z element and rescale x/y to fit the screen
        let screen_space_coords = (ndc_space_coords.truncate() + Vec2::ONE) / 2.0 * size;
        Some(screen_space_coords)
    }
}

#[allow(clippy::type_complexity)]
pub fn camera_system<T: CameraProjection + Component>(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_created_events: EventReader<WindowCreated>,
    mut image_asset_events: EventReader<AssetEvent<Image>>,
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    mut queries: QuerySet<(
        QueryState<(Entity, &mut Camera, &mut T)>,
        QueryState<Entity, Added<Camera>>,
    )>,
) {
    let mut changed_window_ids = Vec::new();
    // handle resize events. latest events are handled first because we only want to resize each
    // window once
    for event in window_resized_events.iter().rev() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        changed_window_ids.push(event.id);
    }

    // handle resize events. latest events are handled first because we only want to resize each
    // window once
    for event in window_created_events.iter().rev() {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        changed_window_ids.push(event.id);
    }

    let mut added_cameras = vec![];
    for entity in &mut queries.q1().iter() {
        added_cameras.push(entity);
    }

    let changed_textures: Vec<_> = image_asset_events
        .iter()
        .map(|event| event.handle())
        .collect();

    for (entity, mut camera, mut camera_projection) in queries.q0().iter_mut() {
        if let Some(size) = camera.target.size(&windows, &images) {
            let render_target_changed = match &camera.target {
                CameraTarget::Window(window_id) => changed_window_ids.contains(window_id),
                CameraTarget::Texture(image) => changed_textures.contains(&image),
            };

            if render_target_changed
                || added_cameras.contains(&entity)
                || camera_projection.is_changed()
            {
                camera_projection.update(size.0, size.1);
                camera.projection_matrix = camera_projection.get_projection_matrix();
                camera.depth_calculation = camera_projection.depth_calculation();
            }
        }
    }
}
