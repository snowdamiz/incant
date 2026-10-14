//! Fixed engine programs; never a project shader passthrough.
mod contract;
mod emit;
mod interpolation;
mod material;
mod material_emit;
mod sprite;
mod text;
mod ui;

use crate::{Artifact, Backend, Bundle, CookError, Result, Stage};
use naga::{ShaderStage, back::pipeline_constants, valid};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Diagnostic,
    Shadow,
    Sprite,
    Ui,
    Text,
    Material,
}
impl Kind {
    pub fn contract(self) -> &'static str {
        match self {
            Self::Material => {
                "incant-material-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,vertex-sizes1,fragment-sizes7;spv=1.0,no-y-flip;hlsl=6.0,special-b5;bounds=restrict;loops=bounded"
            }
            Self::Diagnostic => {
                "incant-diagnostic-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15,stride28,sizes0;spv=1.0,no-y-flip;hlsl=6.0,special-b0;bounds=restrict;loops=bounded"
            }
            Self::Shadow => {
                "incant-shadow-v1;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14i,stride48-112,sizes1;spv=1.0,no-y-flip;hlsl=6.0,camera-b0,material-b1,texture-t0,sampler-indices-t5,special-b2;bounds=restrict;loops=bounded"
            }
            Self::Text => {
                "incant-text-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15i,stride32,sizes2,frame0,node1;spv=1.0,no-y-flip;hlsl=6.0,frame-b0,node-b1,texture-t0,sampler-indices-t1,special-b2;bounds=restrict;loops=bounded"
            }
            Self::Ui => {
                "incant-ui-v1;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15i,stride64,sizes1,frame0;spv=1.0,no-y-flip;hlsl=6.0,frame-b0,texture-t0,sampler-indices-t1,special-b1;bounds=restrict;loops=bounded"
            }
            Self::Sprite => {
                "incant-sprite-v2;interpolation=centroid;naga=29.0.4;wgpu=29.0.4;msl=2.4,pull15v14v,stride12-52,sizes1;spv=1.0,no-y-flip;hlsl=6.0,camera-b0,texture-t0,sampler-indices-t1,special-b1;bounds=restrict;loops=bounded"
            }
        }
    }
}

/// The caller supplies only these engine-owned sources. Exact IR contracts are
/// enforced before the stage-specific backend emission and private embedding.
pub fn cook(kind: Kind, source: &str) -> Result<Bundle> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| CookError::Parse(e.emit_to_string(source)))?;
    let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .map_err(|e| CookError::Validate(e.to_string()))?;
    interpolation::validate(kind, &module)?;
    contract::validate(kind, &module, &info)?;
    let mut artifacts = Vec::with_capacity(6);
    for (stage, name, tag) in [
        (ShaderStage::Vertex, "vertex", Stage::Vertex),
        (ShaderStage::Fragment, "fragment", Stage::Fragment),
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
        include_str!("interpolation.rs"),
        include_str!("emit.rs"),
        include_str!("sprite.rs"),
        include_str!("ui.rs"),
        include_str!("text.rs"),
        include_str!("material.rs"),
        include_str!("material_emit.rs"),
        include_str!("../../Cargo.toml"),
    ]
    .join("\n");
    Ok(Bundle {
        source_hash: Sha256::digest(source.as_bytes()).into(),
        producer_hash: Sha256::digest(producer).into(),
        artifacts,
    })
}

/// Emit only a private build-generated manifest, with the family ABI identity.
pub fn write_bundle(kind: Kind, bundle: &Bundle, directory: &Path) -> Result<()> {
    if bundle.artifacts.len() != 6 {
        return Err(CookError::Contract("geometry artifact count"));
    }
    fs::create_dir_all(directory)?;
    let mut generated = format!(
        "const MANIFEST_CONTRACT: &str = {:?};\nconst SOURCE_HASH: [u8; 32] = {:?};\nconst PRODUCER_HASH: [u8; 32] = {:?};\nconst ARTIFACTS: [Artifact; 6] = [\n",
        kind.contract(),
        bundle.source_hash,
        bundle.producer_hash,
    );
    for artifact in &bundle.artifacts {
        let file = format!("{:?}-{:?}.shader", artifact.backend, artifact.stage);
        fs::write(directory.join(&file), &artifact.bytes)?;
        let hash: [u8; 32] = Sha256::digest(&artifact.bytes).into();
        writeln!(generated, "Artifact {{ backend: wgpu::Backend::{:?}, stage: Stage::{:?}, entry: {:?}, hash: {:?}, bytes: include_bytes!({:?}) }},", artifact.backend, artifact.stage, artifact.entry, hash, file).expect("writing String");
    }
    generated.push_str("];\n");
    fs::write(directory.join("manifest.rs"), generated)?;
    Ok(())
}
