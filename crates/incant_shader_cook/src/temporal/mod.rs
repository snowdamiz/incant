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
    Resolve,
    Copy,
    Upscale,
    Composite,
}
impl Kind {
    pub fn entry(self) -> &'static str {
        match self {
            Self::Motion => "fragment",
            Self::Resolve => "resolve",
            Self::Copy => "copy",
            Self::Upscale => "resolve",
            Self::Composite => "composite",
        }
    }
    pub fn contract(self) -> &'static str {
        match self {
            Self::Motion => {
                "incant-temporal-motion-v1;naga=29.0.4;wgpu=29.0.4;frame144-vf,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Resolve => {
                "incant-temporal-resolve-v1;naga=29.0.4;wgpu=29.0.4;frame64-f,textures4,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Copy => {
                "incant-temporal-copy-v1;naga=29.0.4;wgpu=29.0.4;frame0-f,textures1,mrt1;msl=2.4;hlsl=6.0,special-b0;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
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
            Self::Motion => [
                127, 138, 237, 74, 44, 179, 4, 1, 234, 174, 61, 61, 14, 84, 215, 45, 55, 203, 78,
                56, 117, 181, 85, 243, 52, 52, 213, 232, 18, 54, 99, 185,
            ],
            Self::Resolve => [
                174, 101, 177, 242, 100, 125, 204, 229, 167, 63, 173, 202, 75, 125, 104, 198, 129,
                89, 120, 175, 63, 187, 132, 21, 16, 171, 58, 165, 85, 241, 207, 236,
            ],
            Self::Copy => [
                80, 242, 187, 186, 145, 2, 247, 248, 157, 229, 36, 53, 107, 126, 110, 75, 155, 30,
                76, 144, 236, 172, 231, 165, 177, 193, 110, 229, 153, 238, 75, 223,
            ],
            Self::Upscale => [
                127, 178, 128, 96, 123, 67, 29, 59, 16, 5, 58, 3, 192, 148, 170, 194, 64, 211, 199,
                151, 54, 126, 212, 122, 88, 235, 246, 191, 85, 47, 35, 246,
            ],
            Self::Composite => [
                46, 65, 155, 10, 187, 238, 70, 138, 83, 174, 70, 182, 115, 96, 218, 193, 79, 172,
                207, 51, 7, 233, 213, 70, 144, 180, 95, 28, 221, 61, 114, 122,
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
