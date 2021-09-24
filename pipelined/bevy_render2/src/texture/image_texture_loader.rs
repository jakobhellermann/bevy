use bevy_asset::{
    distill_importer::{ImportedAsset, Importer, ImporterValue},
    util::AssetUuidImporterState,
};
use bevy_reflect::TypeUuid;
use image::ImageFormat;

use crate::texture::image_texture_conversion::image_to_texture;

/// Loader for images that can be read by the `image` crate.
#[derive(Clone, TypeUuid)]
#[uuid = "42d33fcd-1518-4689-9d66-ffc19e92bbfa"]
pub struct ImageTextureLoader(pub ImageFormat);

impl Importer for ImageTextureLoader {
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
    ) -> bevy_asset::distill_importer::Result<ImporterValue> {
        let mut buf = Vec::new();
        source.read_to_end(&mut buf)?;

        let dyn_image = image::load_from_memory_with_format(&buf, self.0)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
        let image = image_to_texture(dyn_image);

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id(),
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(image),
            }],
        })
    }
}

#[allow(dead_code)]
pub(crate) const FILE_EXTENSIONS: &[(&[&str], ImageFormat)] = &[
    (&["png"], ImageFormat::Png),
    (&["dds"], ImageFormat::Dds),
    (&["tga"], ImageFormat::Tga),
    (&["jpg", "jpeg"], ImageFormat::Jpeg),
    (&["bmp"], ImageFormat::Bmp),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_file_extensions() {
        for ext in FILE_EXTENSIONS {
            assert!(image::ImageFormat::from_extension(ext).is_some())
        }
    }
}
