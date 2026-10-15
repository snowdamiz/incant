//! Complete fixed SSAO source and native ABI; no project program loader.
mod contract;
mod emit;
use crate::geometry::material::Variant;
use crate::{Artifact, Backend, CookError, Result, Stage};
use naga::{ShaderStage, back::pipeline_constants, valid};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Prepass,
    Forward,
    Sample,
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bundle {
    kind: Kind,
    pub source_hash: [u8; 32],
    pub producer_hash: [u8; 32],
    pub artifacts: Vec<Artifact>,
}
impl Kind {
    pub fn entry(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
            _ => "fragment",
        }
    }
    pub fn contract(self) -> &'static str {
        match self {
            Self::Prepass => {
                "incant-ssao-prepass-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1;hlsl=6.0,frame-b0,material-b1,base-t0,sampler-indices-t5,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Forward => {
                "incant-ssao-forward-v1;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7,ao-t0,material-t1;hlsl=6.0,ao-t0,material-t1,sampler-indices-t6-t10-t15,special-b5;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Sample => {
                "incant-ssao-sample-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2;msl=2.4,b0,t0-t1;hlsl=6.0,b0,t0-t1,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Horizontal => {
                "incant-ssao-horizontal-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2,accessibility@3;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Vertical => {
                "incant-ssao-vertical-v1;naga=29.0.4;wgpu=29.0.4;uniform736@0,depth@1,normal@2,accessibility@3;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
        }
    }
    pub fn source_hash(self) -> [u8; 32] {
        match self {
            Self::Prepass => [
                103, 182, 72, 163, 165, 188, 39, 192, 11, 26, 140, 32, 77, 44, 29, 64, 113, 215,
                49, 156, 165, 41, 5, 102, 197, 188, 106, 48, 248, 21, 201, 186,
            ],
            Self::Forward => [
                220, 98, 7, 41, 240, 157, 87, 127, 213, 109, 7, 90, 120, 0, 91, 46, 94, 23, 234,
                145, 88, 44, 195, 4, 216, 76, 175, 167, 170, 108, 249, 199,
            ],
            Self::Sample => [
                74, 170, 12, 192, 237, 55, 85, 85, 251, 0, 105, 169, 116, 199, 157, 115, 96, 51,
                127, 62, 235, 222, 190, 228, 58, 135, 222, 88, 190, 148, 198, 182,
            ],
            Self::Horizontal => [
                105, 128, 82, 87, 70, 189, 199, 30, 123, 173, 49, 56, 120, 110, 41, 180, 12, 76, 3,
                222, 0, 250, 69, 140, 128, 154, 145, 187, 41, 200, 78, 208,
            ],
            Self::Vertical => [
                105, 128, 82, 87, 70, 189, 199, 30, 123, 173, 49, 56, 120, 110, 41, 180, 12, 76, 3,
                222, 0, 250, 69, 140, 128, 154, 145, 187, 41, 200, 78, 208,
            ],
        }
    }
    pub(super) fn material_variant(self) -> Option<Variant> {
        match self {
            Self::Prepass => Some(Variant::Prepass),
            Self::Forward => Some(Variant::Occlusion),
            _ => None,
        }
    }
    pub(super) fn bindings(self) -> u32 {
        if self == Self::Sample { 3 } else { 4 }
    }
    pub(super) fn texture(self, binding: u32) -> Option<u8> {
        (binding > 0 && binding < self.bindings()).then(|| (binding - 1) as u8)
    }
    pub(super) fn uniform(self, binding: u32) -> Option<u8> {
        (binding == 0).then_some(0)
    }
}

pub fn cook(kind: Kind, source: &str) -> Result<Bundle> {
    if <[u8; 32]>::from(Sha256::digest(source)) != kind.source_hash() {
        return Err(CookError::Contract("SSAO trusted source identity"));
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
        include_str!("../geometry/mod.rs"),
        include_str!("../geometry/contract.rs"),
        include_str!("../geometry/interpolation.rs"),
        include_str!("../geometry/emit.rs"),
        include_str!("../geometry/sprite.rs"),
        include_str!("../geometry/ui.rs"),
        include_str!("../geometry/text.rs"),
        include_str!("../geometry/material.rs"),
        include_str!("../geometry/material_emit.rs"),
        include_str!("../lib.rs"),
        include_str!("../audit.rs"),
        include_str!("../../Cargo.toml"),
        include_str!("../../../incant_render/src/ambient_occlusion/source.rs"),
    ]
    .join("\n");
    Ok(Bundle {
        kind,
        source_hash: kind.source_hash(),
        producer_hash: Sha256::digest(producer).into(),
        artifacts,
    })
}

pub fn write_bundle(kind: Kind, bundle: &Bundle, directory: &Path) -> Result<()> {
    if bundle.kind != kind
        || bundle.source_hash != kind.source_hash()
        || bundle.producer_hash == [0; 32]
        || bundle.artifacts.len() != 6
    {
        return Err(CookError::Contract("SSAO bundle identity/count"));
    }
    for backend in [Backend::Metal, Backend::Vulkan, Backend::Dx12] {
        for stage in [Stage::Vertex, Stage::Fragment] {
            if bundle
                .artifacts
                .iter()
                .filter(|a| a.backend == backend && a.stage == stage)
                .count()
                != 1
            {
                return Err(CookError::Contract("SSAO bundle stage membership"));
            }
        }
    }
    fs::create_dir_all(directory)?;
    let mut generated = format!(
        "const MANIFEST_CONTRACT: &str = {:?};\nconst SOURCE_HASH: [u8; 32] = {:?};\nconst PRODUCER_HASH: [u8; 32] = {:?};\nconst ARTIFACTS: [Artifact; 6] = [\n",
        kind.contract(),
        bundle.source_hash,
        bundle.producer_hash
    );
    for artifact in &bundle.artifacts {
        let file = format!("{:?}-{:?}.shader", artifact.backend, artifact.stage);
        fs::write(directory.join(&file), &artifact.bytes)?;
        let hash: [u8; 32] = Sha256::digest(&artifact.bytes).into();
        writeln!(generated, "Artifact {{ backend: wgpu::Backend::{:?}, stage: Stage::{:?}, entry: {:?}, hash: {:?}, bytes: include_bytes!({:?}) }},", artifact.backend, artifact.stage, artifact.entry, hash, file).expect("writing String");
    }
    generated.push_str("];\n");
    fs::write(directory.join("manifest.rs"), generated)?;
    crate::audit::write(
        directory,
        kind.contract(),
        &bundle.source_hash,
        &bundle.producer_hash,
        bundle.artifacts.iter().map(|a| {
            (
                a.backend,
                match a.stage {
                    Stage::Vertex => "Vertex",
                    Stage::Fragment => "Fragment",
                },
                a.entry.as_str(),
                a.bytes.as_slice(),
            )
        }),
    )
}
