mod command;
mod dynamic_scene;
mod scene;
mod scene_loader;
mod scene_spawner;
pub mod serde;

use bevy_reflect::TypeRegistryArc;
pub use command::*;
pub use dynamic_scene::*;
pub use scene::*;
pub use scene_loader::*;
pub use scene_spawner::*;

use once_cell::sync::OnceCell;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        DynamicScene, Scene, SceneSpawner, SpawnSceneAsChildCommands, SpawnSceneCommands,
    };
}

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::{schedule::ExclusiveSystemDescriptorCoercion, system::IntoExclusiveSystem};

#[derive(Default)]
pub struct ScenePlugin;

static TYPE_REGISTRY: OnceCell<TypeRegistryArc> = OnceCell::new();

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<SceneLoader>(&["scn", "scn.ron"])
            .add_asset::<DynamicScene>()
            .add_asset_non_deserialize::<Scene>()
            .init_resource::<SceneSpawner>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                scene_spawner_system.exclusive_system().at_end(),
            );

        TYPE_REGISTRY.get_or_init(|| app.world.get_resource::<TypeRegistryArc>().unwrap().clone());
    }
}
