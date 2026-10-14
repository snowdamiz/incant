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
    MotionOwner,
    Depth,
    Resolve,
    Copy,
}
impl Kind {
    pub fn entry(self) -> &'static str {
        match self {
            Self::Motion | Self::MotionOwner => "fragment",
            Self::Depth => "depth_only",
            Self::Resolve => "resolve",
            Self::Copy => "copy",
        }
    }
    pub fn contract(self) -> &'static str {
        match self {
            Self::MotionOwner => {
                "incant-temporal-motion-owner-v1;naga=29.0.4;wgpu=29.0.4;frame144-vf,material64-f,5textures-samplers;mrt=rgba32f-r32u;owner=uint-loc12-offset68-flat3;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Depth => {
                "incant-temporal-depth-v1;naga=29.0.4;wgpu=29.0.4;frame144-v,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
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
        }
    }
    /// Exact reviewed source identity, including all shader arithmetic. Interface
    /// checks alone must not authorize a changed trusted passthrough program.
    pub fn source_hash(self) -> [u8; 32] {
        match self {
            Self::MotionOwner => [
                102, 9, 205, 235, 39, 74, 200, 74, 255, 222, 133, 196, 199, 126, 142, 116, 65, 223,
                251, 183, 72, 224, 124, 45, 112, 156, 179, 170, 141, 96, 109, 0,
            ],
            Self::Motion | Self::Depth => [
                30, 190, 25, 208, 56, 144, 101, 74, 214, 210, 50, 136, 31, 151, 182, 253, 29, 74,
                10, 195, 201, 189, 178, 150, 17, 209, 183, 33, 168, 48, 71, 123,
            ],
            Self::Resolve => [
                78, 127, 30, 229, 3, 224, 91, 199, 242, 193, 38, 174, 131, 176, 89, 136, 31, 233,
                11, 153, 83, 5, 7, 190, 94, 169, 247, 240, 55, 182, 139, 206,
            ],
            Self::Copy => [
                9, 110, 232, 158, 203, 92, 145, 18, 222, 234, 85, 147, 90, 194, 218, 11, 236, 79,
                215, 243, 186, 96, 58, 160, 76, 135, 91, 164, 141, 235, 36, 139,
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
