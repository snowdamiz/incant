//! Fixed temporal engine programs. No project supplied passthrough shaders.
mod contract;
mod emit;
use crate::{Artifact, Backend, Bundle, CookError, Result, Stage};
use naga::{ShaderStage, back::pipeline_constants, valid};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs, path::Path};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Motion,
    Depth,
    Resolve,
    Copy,
    Upscale,
    Composite,
    MetalFxPack,
    MetalFxNormalize,
}
impl Kind {
    pub fn entry(self) -> &'static str {
        match self {
            Self::Motion => "fragment",
            Self::Depth => "depth_only",
            Self::MetalFxPack => "pack",
            Self::MetalFxNormalize => "normalize",
            Self::Resolve => "resolve",
            Self::Copy => "copy",
            Self::Upscale => "resolve",
            Self::Composite => "composite",
        }
    }
    pub fn contract(self) -> &'static str {
        match self {
            Self::Depth => {
                "incant-temporal-depth-v1;naga=29.0.4;wgpu=29.0.4;frame144-v,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::MetalFxPack => {
                "incant-metalfx-pack-v1;naga=29.0.4;wgpu=29.0.4;frame96-f,textures3,mrt4;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::MetalFxNormalize => {
                "incant-metalfx-normalize-v1;naga=29.0.4;wgpu=29.0.4;frame96-f,textures3,mrt2;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Motion => {
                "incant-temporal-motion-v1;naga=29.0.4;wgpu=29.0.4;frame144-vf,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Resolve => {
                "incant-temporal-resolve-v1;naga=29.0.4;wgpu=29.0.4;frame64-f,textures4,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Copy => {
                "incant-temporal-copy-v2;naga=29.0.4;wgpu=29.0.4;frame64-f@1,textures1@0,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Upscale => {
                "incant-temporal-upscale-v1;naga=29.0.4;wgpu=29.0.4;frame96-f,textures5,mrt2;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Composite => {
                "incant-temporal-composite-v1;naga=29.0.4;wgpu=29.0.4;frame96-f,textures2,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
        }
    }
    /// Exact reviewed source identity, including all shader arithmetic. Interface
    /// checks alone must not authorize a changed trusted passthrough program.
    pub fn source_hash(self) -> [u8; 32] {
        match self {
            Self::MetalFxPack => [
                13, 74, 30, 80, 142, 80, 238, 126, 110, 228, 59, 26, 142, 109, 50, 79, 221, 92, 44,
                17, 7, 203, 57, 184, 35, 132, 69, 109, 239, 63, 154, 177,
            ],
            Self::MetalFxNormalize => [
                4, 205, 39, 235, 243, 86, 233, 85, 144, 240, 0, 117, 131, 197, 13, 73, 167, 186,
                86, 68, 83, 120, 116, 79, 66, 62, 0, 9, 88, 194, 65, 166,
            ],
            Self::Motion | Self::Depth => [
                30, 190, 25, 208, 56, 144, 101, 74, 214, 210, 50, 136, 31, 151, 182, 253, 29, 74,
                10, 195, 201, 189, 178, 150, 17, 209, 183, 33, 168, 48, 71, 123,
            ],
            Self::Resolve => [
                153, 12, 98, 101, 217, 17, 188, 80, 93, 110, 82, 119, 232, 27, 43, 17, 196, 37, 45,
                62, 105, 25, 245, 124, 26, 228, 140, 210, 9, 169, 224, 223,
            ],
            Self::Copy => [
                120, 14, 57, 169, 35, 243, 30, 146, 45, 18, 12, 198, 72, 176, 5, 86, 104, 152, 194,
                239, 158, 85, 124, 225, 194, 148, 225, 206, 172, 246, 21, 4,
            ],
            Self::Upscale => [
                74, 5, 87, 196, 159, 13, 190, 192, 49, 59, 251, 215, 27, 237, 98, 104, 127, 114,
                46, 184, 161, 62, 100, 20, 139, 172, 72, 246, 136, 17, 230, 242,
            ],
            Self::Composite => [
                180, 98, 116, 131, 198, 85, 140, 185, 102, 44, 185, 178, 160, 77, 6, 148, 11, 255,
                117, 137, 129, 57, 5, 24, 129, 40, 79, 22, 63, 68, 53, 62,
            ],
        }
    }
}
pub fn cook(kind: Kind, source: &str) -> Result<Bundle> {
    if <[u8; 32]>::from(Sha256::digest(source)) != kind.source_hash() {
        return Err(CookError::Contract("temporal trusted source identity"));
    }
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| CookError::Parse(e.emit_to_string(source)))?;
    let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .map_err(|e| CookError::Validate(e.to_string()))?;
    contract::validate(kind, &module, &info)?;
    let mut artifacts = Vec::with_capacity(6);
    for (stage, name, tag) in [
        (ShaderStage::Vertex, "vertex", Stage::Vertex),
        (ShaderStage::Fragment, kind.entry(), Stage::Fragment),
    ] {
        let (compact, compact_info) = pipeline_constants::process_overrides(
            &module,
            &info,
            Some((stage, name)),
            &Default::default(),
        )
        .map_err(|e| CookError::Emit(e.to_string()))?;
        for backend in [Backend::Metal, Backend::Vulkan, Backend::Dx12] {
            let (bytes, entry) =
                emit::emit(kind, &compact, &compact_info, &module, stage, name, backend)?;
            artifacts.push(Artifact {
                backend,
                stage: tag,
                entry,
                bytes,
            });
        }
    }
    let producer = [
        include_str!("mod.rs"),
        include_str!("contract.rs"),
        include_str!("emit.rs"),
        include_str!("../../Cargo.toml"),
    ]
    .join("\n");
    Ok(Bundle {
        source_hash: kind.source_hash(),
        producer_hash: Sha256::digest(producer).into(),
        artifacts,
    })
}
pub fn write_bundle(kind: Kind, bundle: &Bundle, directory: &Path) -> Result<()> {
    if bundle.artifacts.len() != 6 || bundle.source_hash != kind.source_hash() {
        return Err(CookError::Contract("temporal bundle identity"));
    }
    fs::create_dir_all(directory)?;
    let mut generated = format!(
        "const MANIFEST_CONTRACT: &str = {:?};\nconst SOURCE_HASH: [u8;32] = {:?};\nconst PRODUCER_HASH: [u8;32] = {:?};\nconst ARTIFACTS: [Artifact;6] = [\n",
        kind.contract(),
        bundle.source_hash,
        bundle.producer_hash
    );
    for a in &bundle.artifacts {
        let file = format!("{:?}-{:?}.shader", a.backend, a.stage);
        fs::write(directory.join(&file), &a.bytes)?;
        let hash: [u8; 32] = Sha256::digest(&a.bytes).into();
        writeln!(generated,"Artifact {{backend:wgpu::Backend::{:?},stage:Stage::{:?},entry:{:?},hash:{:?},bytes:include_bytes!({:?})}},",a.backend,a.stage,a.entry,hash,file).expect("String");
    }
    generated.push_str("];\n");
    fs::write(directory.join("manifest.rs"), generated)?;
    Ok(())
}
