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
    Upscale,
    UpscaleOwner,
    Composite,
    MetalFxPack,
    MetalFxNormalize,
}
impl Kind {
    pub fn entry(self) -> &'static str {
        match self {
            Self::Motion | Self::MotionOwner => "fragment",
            Self::Depth => "depth_only",
            Self::MetalFxPack => "pack",
            Self::MetalFxNormalize => "normalize",
            Self::Resolve => "resolve",
            Self::Copy => "copy",
            Self::Upscale | Self::UpscaleOwner => "resolve",
            Self::Composite => "composite",
        }
    }
    pub fn contract(self) -> &'static str {
        match self {
            Self::MotionOwner => {
                "incant-temporal-motion-owner-v2;naga=29.0.4;wgpu=29.0.4;frame160-vf,material64-f,5textures-samplers;mrt=rgba32f-r32u;owner=uint-loc12-offset68-flat3;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Depth => {
                "incant-temporal-depth-v2;naga=29.0.4;wgpu=29.0.4;frame160-v,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::MetalFxPack => {
                "incant-metalfx-pack-v1;naga=29.0.4;wgpu=29.0.4;frame96-f,textures3,mrt4;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::MetalFxNormalize => {
                "incant-metalfx-normalize-v1;naga=29.0.4;wgpu=29.0.4;frame96-f,textures3,mrt2;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Motion => {
                "incant-temporal-motion-v2;naga=29.0.4;wgpu=29.0.4;frame160-vf,material64-f,5textures-samplers;msl=2.4,pulls15-14-13-12,strides48-48-112-80,lengths1,frame0,material1;hlsl=6.0,b0-b1,sampler-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Resolve => {
                "incant-temporal-resolve-v1;naga=29.0.4;wgpu=29.0.4;frame64-f,textures4,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Copy => {
                "incant-temporal-copy-v2;naga=29.0.4;wgpu=29.0.4;frame64-f@1,textures1@0,mrt1;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::UpscaleOwner => {
                "incant-temporal-upscale-owner-v2;naga=29.0.4;wgpu=29.0.4;frame96-f,changes12528-f,6float-depth+2uint-textures,mrt=rgba16f-r32f-rg32u-rg16f;msl=2.4,frame0,changes1;hlsl=6.0,b0-b1,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Upscale => {
                "incant-temporal-upscale-v2;naga=29.0.4;wgpu=29.0.4;frame96-f,textures6,mrt3;msl=2.4;hlsl=6.0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
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
                143, 8, 182, 224, 244, 66, 228, 150, 137, 22, 166, 202, 74, 160, 152, 161, 199, 11,
                175, 78, 227, 58, 156, 145, 144, 138, 73, 38, 57, 22, 129, 241,
            ],
            Self::MetalFxNormalize => [
                9, 35, 231, 186, 77, 197, 175, 222, 3, 220, 42, 193, 68, 229, 192, 172, 85, 108,
                209, 115, 118, 137, 112, 213, 84, 222, 124, 208, 129, 96, 117, 181,
            ],
            Self::MotionOwner => [
                162, 101, 152, 121, 205, 89, 105, 134, 51, 210, 143, 93, 183, 248, 164, 240,
                134, 56, 47, 38, 14, 230, 145, 29, 227, 18, 165, 178, 209, 12, 66, 219,
            ], // Claude 0073 r4 MASK LOD bias
            Self::Motion | Self::Depth => [
                230, 75, 74, 28, 192, 227, 147, 199, 82, 237, 222, 186, 138, 247, 146, 181,
                157, 185, 202, 195, 206, 41, 145, 249, 255, 199, 241, 126, 30, 33, 100, 208,
            ], // Claude 0073 r4 MASK LOD bias
            Self::Resolve => [
                225, 175, 83, 86, 90, 41, 159, 106, 3, 161, 58, 62, 224, 90, 3, 123, 84, 14, 36,
                203, 43, 11, 237, 5, 76, 238, 184, 210, 212, 132, 235, 241,
            ],
            Self::Copy => [
                9, 110, 232, 158, 203, 92, 145, 18, 222, 234, 85, 147, 90, 194, 218, 11, 236, 79,
                215, 243, 186, 96, 58, 160, 76, 135, 91, 164, 141, 235, 36, 139,
            ],
            Self::UpscaleOwner => [
                8, 151, 191, 210, 237, 94, 143, 139, 143, 159, 192, 185, 157, 62, 44, 82,
                143, 178, 158, 103, 118, 53, 225, 136, 218, 131, 84, 5, 255, 237, 235, 129,
            ], // Claude 0073 r4 evidence estimator, full-support provenance
            Self::Upscale => [
                222, 77, 248, 4, 192, 253, 147, 253, 19, 200, 73, 87, 92, 191, 76, 79,
                198, 24, 75, 170, 153, 136, 173, 178, 28, 83, 82, 115, 207, 209, 211, 163,
            ], // Claude 0073 r4 evidence estimator
            Self::Composite => [
                129, 207, 231, 197, 156, 114, 85, 211, 53, 86, 189, 212, 33, 103, 207, 218, 245,
                203, 31, 62, 204, 242, 36, 180, 61, 83, 155, 186, 117, 165, 155, 55,
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
