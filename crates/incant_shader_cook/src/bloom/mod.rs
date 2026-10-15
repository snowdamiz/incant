//! Fixed bloom programs only; no project-controlled source or shader loader.
mod contract;
mod emit;
use crate::{Artifact, Backend, CookError, Result, Stage};
use naga::{ShaderStage, back::pipeline_constants, valid};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Prefilter,
    Downsample,
    Upsample,
    Display,
}

/// Preserve the chosen entry family even when three programs share source.
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
            Self::Prefilter => "first",
            Self::Downsample => "down",
            Self::Upsample => "up",
            Self::Display => "fragment",
        }
    }
    pub fn contract(self) -> &'static str {
        match self {
            Self::Prefilter => {
                "incant-bloom-prefilter-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Downsample => {
                "incant-bloom-downsample-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Upsample => {
                "incant-bloom-upsample-v1;naga=29.0.4;wgpu=29.0.4;textures0-1,uniform48@2;msl=2.4,t0-t1,b0;hlsl=6.0,t0-t1,b0,special-b1;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
            Self::Display => {
                "incant-bloom-display-v1;naga=29.0.4;wgpu=29.0.4;textures0-2,uniform80@1,uniform16@3;msl=2.4,t0-t1,b0-b1;hlsl=6.0,t0-t1,b0-b1,special-b2;spv=1.0,no-y-flip;bounds=restrict;loops=bounded"
            }
        }
    }
    pub fn source_hash(self) -> [u8; 32] {
        if self == Self::Display {
            [
                78, 98, 152, 66, 102, 34, 233, 128, 158, 199, 69, 28, 117, 70, 191, 55, 13, 132,
                97, 88, 87, 61, 32, 181, 93, 174, 82, 113, 7, 200, 104, 188,
            ]
        } else {
            [
                183, 7, 34, 133, 167, 18, 122, 242, 210, 11, 114, 219, 80, 170, 140, 238, 94, 65,
                65, 82, 170, 32, 195, 102, 122, 131, 108, 197, 82, 84, 223, 199,
            ]
        }
    }
    pub(super) fn bindings(self) -> u32 {
        if self == Self::Display { 4 } else { 3 }
    }
    pub(super) fn texture(self, binding: u32) -> Option<u8> {
        match (self, binding) {
            (_, 0) => Some(0),
            (Self::Display, 2) | (Self::Prefilter | Self::Downsample | Self::Upsample, 1) => {
                Some(1)
            }
            _ => None,
        }
    }
    pub(super) fn uniform(self, binding: u32) -> Option<u8> {
        match (self, binding) {
            (Self::Display, 1) | (Self::Prefilter | Self::Downsample | Self::Upsample, 2) => {
                Some(0)
            }
            (Self::Display, 3) => Some(1),
            _ => None,
        }
    }
}

pub fn cook(kind: Kind, source: &str) -> Result<Bundle> {
    if <[u8; 32]>::from(Sha256::digest(source)) != kind.source_hash() {
        return Err(CookError::Contract("bloom trusted source identity"));
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
        include_str!("../audit.rs"),
        include_str!("../../Cargo.toml"),
        include_str!("../../../incant_render/src/bloom/source.rs"),
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
        return Err(CookError::Contract("bloom bundle identity/count"));
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
                return Err(CookError::Contract("bloom bundle stage membership"));
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
