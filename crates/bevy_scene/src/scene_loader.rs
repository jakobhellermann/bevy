use anyhow::Result;
use bevy_asset::distill_importer::ImportedAsset;
use bevy_ecs::world::{FromWorld, World};
use bevy_reflect::TypeRegistryArc;

use bevy_asset::prelude::*;

use bevy_asset::{
    distill_importer::{Importer, ImporterValue},
    util::AssetUuidImporterState,
};
use serde::de::DeserializeSeed;

use crate::serde::SceneDeserializer;

#[derive(Debug, TypeUuid)]
#[uuid = "47e6fb05-1336-4d7e-94f0-7ba77c58f1f4"]
pub struct SceneLoader {
    type_registry: TypeRegistryArc,
}

impl FromWorld for SceneLoader {
    fn from_world(world: &mut World) -> Self {
        let type_registry = world.get_resource::<TypeRegistryArc>().unwrap();
        SceneLoader {
            type_registry: (&*type_registry).clone(),
        }
    }
}

impl Importer for SceneLoader {
    fn version_static() -> u32
    where
        Self: Sized,
    {
        1
    }

    fn version(&self) -> u32 {
        Self::version_static()
    }

    type Options = ();
    type State = AssetUuidImporterState;

    fn import(
        &self,
        _: &mut bevy_asset::distill_importer::ImportOp,
        source: &mut dyn std::io::Read,
        _: &Self::Options,
        state: &mut Self::State,
    ) -> Result<ImporterValue, bevy_asset::distill_importer::Error> {
        let err = |e| Box::new(e) as Box<dyn std::error::Error + Send>;

        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let mut deserializer = ron::de::Deserializer::from_bytes(&bytes).map_err(err)?;
        let scene_deserializer = SceneDeserializer {
            type_registry: &*self.type_registry.read(),
        };
        let scene = scene_deserializer
            .deserialize(&mut deserializer)
            .map_err(err)?;

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id(),
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(scene),
            }],
        })
    }
}
