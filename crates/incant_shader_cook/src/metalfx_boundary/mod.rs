//! Fixed engine-owned MetalFX boundary programs, not vendor implementation.
mod contract;
mod emit;
use crate::{Artifact, Backend, CookError, Result, Stage};
use naga::{ShaderStage, back::pipeline_constants, valid};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs, path::Path};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Pack,
    Normalize,
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
            Self::Pack => "pack",
            Self::Normalize => "normalize",
        }
    }
    pub fn contract(self) -> &'static str {
        match self {
            Self::Pack => {
                "incant-metalfx-pack-v1;naga=29.0.4;wgpu=29.0.4;frame96-f@0,textures2@1-2,depth@3;mrt=rgba16f-r32f-rg16f-r8;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Normalize => {
                "incant-metalfx-normalize-v1;naga=29.0.4;wgpu=29.0.4;frame96-f@0,textures2@1-2,depth@3;mrt=rgba16f;msl=2.4,b0,t0-t2;hlsl=6.0,b0,t0-t2,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
        }
    }
    pub fn source_hash(self) -> [u8; 32] {
        match self {
            Self::Pack => [
                110, 201, 124, 107, 36, 251, 137, 203, 147, 212, 73, 105, 205, 235, 162, 92, 22,
                178, 46, 123, 178, 53, 116, 249, 92, 143, 190, 41, 116, 107, 14, 251,
            ],
            Self::Normalize => [
                237, 36, 208, 150, 132, 154, 51, 197, 212, 192, 60, 162, 173, 166, 242, 185, 40,
                17, 68, 54, 127, 104, 144, 32, 110, 242, 188, 101, 213, 134, 123, 70,
            ],
        }
    }
}
pub fn cook(kind: Kind, source: &str) -> Result<Bundle> {
    if <[u8; 32]>::from(Sha256::digest(source)) != kind.source_hash() {
        return Err(CookError::Contract(
            "MetalFX boundary trusted source identity",
        ));
    }
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|error| CookError::Parse(error.emit_to_string(source)))?;
    let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .map_err(|error| CookError::Validate(error.to_string()))?;
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
        .map_err(|error| CookError::Emit(error.to_string()))?;
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
    Ok(Bundle {
        kind,
        source_hash: kind.source_hash(),
        producer_hash: producer_hash(),
        artifacts,
    })
}
fn producer_hash() -> [u8; 32] {
    Sha256::digest(
        [
            include_str!("mod.rs"),
            include_str!("contract.rs"),
            include_str!("emit.rs"),
            include_str!("../audit.rs"),
            include_str!("../../Cargo.toml"),
            include_str!("../../../incant_render/src/metalfx/source.rs"),
        ]
        .join("\n"),
    )
    .into()
}
pub fn write_bundle(kind: Kind, bundle: &Bundle, directory: &Path) -> Result<()> {
    if bundle.kind != kind
        || bundle.source_hash != kind.source_hash()
        || bundle.producer_hash != producer_hash()
        || bundle.artifacts.len() != 6
    {
        return Err(CookError::Contract("MetalFX boundary bundle identity"));
    }
    for backend in [Backend::Metal, Backend::Vulkan, Backend::Dx12] {
        for stage in [Stage::Vertex, Stage::Fragment] {
            if bundle
                .artifacts
                .iter()
                .filter(|artifact| artifact.backend == backend && artifact.stage == stage)
                .count()
                != 1
            {
                return Err(CookError::Contract("MetalFX boundary stage membership"));
            }
        }
    }
    fs::create_dir_all(directory)?;
    let mut manifest = format!(
        "const MANIFEST_CONTRACT: &str = {:?};\nconst SOURCE_HASH: [u8; 32] = {:?};\nconst PRODUCER_HASH: [u8; 32] = {:?};\nconst ARTIFACTS: [Artifact; 6] = [\n",
        kind.contract(),
        bundle.source_hash,
        bundle.producer_hash
    );
    for artifact in &bundle.artifacts {
        let file = format!("{:?}-{:?}.shader", artifact.backend, artifact.stage);
        fs::write(directory.join(&file), &artifact.bytes)?;
        let hash: [u8; 32] = Sha256::digest(&artifact.bytes).into();
        writeln!(manifest,"Artifact {{ backend: wgpu::Backend::{:?}, stage: Stage::{:?}, entry: {:?}, hash: {:?}, bytes: include_bytes!({:?}) }},",artifact.backend,artifact.stage,artifact.entry,hash,file).expect("String");
    }
    manifest.push_str("];\n");
    fs::write(directory.join("manifest.rs"), manifest)?;
    crate::audit::write(
        directory,
        kind.contract(),
        &bundle.source_hash,
        &bundle.producer_hash,
        bundle.artifacts.iter().map(|artifact| {
            (
                artifact.backend,
                match artifact.stage {
                    Stage::Vertex => "Vertex",
                    Stage::Fragment => "Fragment",
                },
                artifact.entry.as_str(),
                artifact.bytes.as_slice(),
            )
        }),
    )
}
#[cfg(test)]
#[path = "../../../incant_render/src/metalfx/source.rs"]
mod trusted_source;
#[cfg(test)]
fn source_for_test(kind: Kind) -> String {
    trusted_source::source(kind == Kind::Normalize)
}
