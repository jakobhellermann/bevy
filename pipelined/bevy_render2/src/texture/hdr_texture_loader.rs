use crate::texture::{Image, TextureFormatPixelInfo};
use bevy_asset::{
    distill_importer::{ImportedAsset, Importer, ImporterValue},
    util::AssetUuidImporterState,
};
use wgpu::{Extent3d, TextureDimension, TextureFormat};

/// Loads HDR textures as Texture assets
#[derive(Clone, Default)]
pub struct HdrTextureLoader;

impl Importer for HdrTextureLoader {
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
    ) -> bevy_asset::distill_importer::Result<bevy_asset::distill_importer::ImporterValue> {
        let err = |e| Box::new(e) as Box<dyn std::error::Error + Send>;

        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;

        let format = TextureFormat::Rgba32Float;
        debug_assert_eq!(
            format.pixel_size(),
            4 * 4,
            "Format should have 32bit x 4 size"
        );

        let decoder = image::hdr::HdrDecoder::new(bytes.as_slice()).map_err(err)?;
        let info = decoder.metadata();
        let rgb_data = decoder.read_image_hdr().map_err(err)?;
        let mut rgba_data = Vec::with_capacity(rgb_data.len() * format.pixel_size());

        for rgb in rgb_data {
            let alpha = 1.0f32;

            rgba_data.extend_from_slice(&rgb.0[0].to_ne_bytes());
            rgba_data.extend_from_slice(&rgb.0[1].to_ne_bytes());
            rgba_data.extend_from_slice(&rgb.0[2].to_ne_bytes());
            rgba_data.extend_from_slice(&alpha.to_ne_bytes());
        }

        let texture = Image::new(
            Extent3d {
                width: info.width,
                height: info.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba_data,
            format,
        );

        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id(),
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(texture),
            }],
        })
    }
}
