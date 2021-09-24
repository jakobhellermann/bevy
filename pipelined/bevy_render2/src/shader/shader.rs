use bevy_asset::{
    distill_importer::{ImportedAsset, Importer, ImporterValue},
    util::AssetUuidImporterState,
};
use bevy_reflect::{TypeUuid, Uuid};
use bevy_utils::tracing::error;
use naga::{valid::ModuleInfo, Module};
use std::{borrow::Cow, marker::Copy};
use thiserror::Error;
use wgpu::{ShaderModuleDescriptor, ShaderSource};

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub struct ShaderId(Uuid);

impl ShaderId {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        ShaderId(Uuid::new_v4())
    }
}

#[derive(Error, Debug)]
pub enum ShaderReflectError {
    #[error(transparent)]
    WgslParse(#[from] naga::front::wgsl::ParseError),
    #[error("GLSL Parse Error: {0:?}")]
    GlslParse(Vec<naga::front::glsl::Error>),
    #[error(transparent)]
    SpirVParse(#[from] naga::front::spv::Error),
    #[error(transparent)]
    Validation(#[from] naga::valid::ValidationError),
}

/// A shader, as defined by its [ShaderSource] and [ShaderStage]
#[derive(Debug, TypeUuid, serde::Serialize, serde::Deserialize)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub enum Shader {
    Wgsl(Cow<'static, str>),
    Glsl(Cow<'static, str>, naga::ShaderStage),
    SpirV(Vec<u8>),
    // TODO: consider the following
    // PrecompiledSpirVMacros(HashMap<HashSet<String>, Vec<u32>>)
    // NagaModule(Module) ... Module impls Serialize/Deserialize
}

pub struct ShaderReflection {
    pub module: Module,
    pub module_info: ModuleInfo,
}

impl ShaderReflection {
    pub fn get_spirv(&self) -> Result<Vec<u32>, naga::back::spv::Error> {
        naga::back::spv::write_vec(
            &self.module,
            &self.module_info,
            &naga::back::spv::Options {
                flags: naga::back::spv::WriterFlags::empty(),
                ..naga::back::spv::Options::default()
            },
            None,
        )
    }

    pub fn get_wgsl(&self) -> Result<String, naga::back::wgsl::Error> {
        naga::back::wgsl::write_string(&self.module, &self.module_info)
    }
}

impl Shader {
    pub fn reflect(&self) -> Result<ShaderReflection, ShaderReflectError> {
        let module = match &self {
            // TODO: process macros here
            Shader::Wgsl(source) => naga::front::wgsl::parse_str(source)?,
            Shader::Glsl(source, shader_stage) => {
                let mut parser = naga::front::glsl::Parser::default();
                parser
                    .parse(&naga::front::glsl::Options::from(*shader_stage), source)
                    .map_err(ShaderReflectError::GlslParse)?
            }
            Shader::SpirV(source) => naga::front::spv::parse_u8_slice(
                source,
                &naga::front::spv::Options {
                    adjust_coordinate_space: false,
                    ..naga::front::spv::Options::default()
                },
            )?,
        };
        let module_info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::default(),
            naga::valid::Capabilities::default(),
        )
        .validate(&module)?;

        Ok(ShaderReflection {
            module,
            module_info,
        })
    }

    pub fn from_wgsl(source: impl Into<Cow<'static, str>>) -> Shader {
        Shader::Wgsl(source.into())
    }

    pub fn from_glsl(source: impl Into<Cow<'static, str>>, stage: naga::ShaderStage) -> Shader {
        Shader::Glsl(source.into(), stage)
    }

    pub fn from_spirv(source: Vec<u8>) -> Shader {
        Shader::SpirV(source)
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "779c30ff-566a-4322-9278-111db2b93756"]
pub struct SpvShaderLoader;

impl Importer for SpvShaderLoader {
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
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        let shader = Shader::from_spirv(bytes);
        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id(),
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader),
            }],
        })
    }
}

#[derive(TypeUuid, Default)]
#[uuid = "c0b3811a-e22e-453a-af33-19b9337e89e1"]
pub struct WgslShaderLoader;

impl Importer for WgslShaderLoader {
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
        let mut wgsl = String::new();
        source.read_to_string(&mut wgsl)?;
        let shader = Shader::from_wgsl(wgsl);
        Ok(ImporterValue {
            assets: vec![ImportedAsset {
                id: state.id(),
                search_tags: vec![],
                build_deps: vec![],
                load_deps: vec![],
                build_pipeline: None,
                asset_data: Box::new(shader),
            }],
        })
    }
}

impl<'a> From<&'a Shader> for ShaderModuleDescriptor<'a> {
    fn from(shader: &'a Shader) -> Self {
        ShaderModuleDescriptor {
            label: None,
            source: match shader {
                Shader::Wgsl(source) => ShaderSource::Wgsl(source.clone()),
                Shader::Glsl(_source, _stage) => {
                    let reflection = shader.reflect().unwrap();
                    let wgsl = reflection.get_wgsl().unwrap();
                    ShaderSource::Wgsl(wgsl.into())
                }
                Shader::SpirV(_) => {
                    // TODO: we can probably just transmute the u8 array to u32?
                    let reflection = shader.reflect().unwrap();
                    let spirv = reflection.get_spirv().unwrap();
                    ShaderSource::SpirV(Cow::Owned(spirv))
                }
            },
        }
    }
}
