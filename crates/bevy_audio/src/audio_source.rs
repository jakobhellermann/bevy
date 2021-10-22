use anyhow::Result;
use bevy_asset::{
    distill_importer::{ImportedAsset, Importer, ImporterValue},
    util::AssetUuidImporterState,
};
use bevy_reflect::TypeUuid;
use serde::{Deserialize, Serialize};
use std::{io::Cursor, sync::Arc};

/// A source of audio data
#[derive(Debug, Clone, TypeUuid, Serialize, Deserialize)]
#[uuid = "7a14806a-672b-443b-8d16-4f18afefa463"]
pub struct AudioSource {
    pub bytes: Arc<[u8]>,
}

impl AsRef<[u8]> for AudioSource {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

/// Loads mp3 files as [AudioSource] [Assets](bevy_asset::Assets)
#[derive(TypeUuid, Default)]
#[uuid = "0fe6232a-4e1f-4d24-abf0-0ca3863e740c"]
pub struct Mp3Loader;

impl Importer for Mp3Loader {
    /*fn load(&self, bytes: &[u8], load_context: &mut LoadContext) -> BoxedFuture<Result<()>> {
        load_context.set_default_asset(LoadedAsset::new(AudioSource {
            bytes: bytes.into(),
        }));
        Box::pin(async move { Ok(()) })
    }

    fn extensions(&self) -> &[&str] {
        &[
            #[cfg(feature = "mp3")]
            "mp3",
            #[cfg(feature = "flac")]
            "flac",
            #[cfg(feature = "wav")]
            "wav",
            #[cfg(feature = "vorbis")]
            "ogg",
        ]
    }
    */

    fn version_static() -> u32
    where
        Self: Sized,
    {
        1
    }

    fn version(&self) -> u32 {
        1
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
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        let source = AudioSource {
            bytes: bytes.into(),
        };
        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id(),
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(source),
            }],
        })
    }
}

pub trait Decodable: Send + Sync + 'static {
    type Decoder;

    fn decoder(&self) -> Self::Decoder;
}

impl Decodable for AudioSource {
    type Decoder = rodio::Decoder<Cursor<AudioSource>>;

    fn decoder(&self) -> Self::Decoder {
        rodio::Decoder::new(Cursor::new(self.clone())).unwrap()
    }
}
