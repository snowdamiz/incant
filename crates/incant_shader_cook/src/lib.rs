//! Host-only cooking of the renderer's fixed display-output shader.
//! This is not a project-shader compiler or a general wgpu layout compiler.
pub mod compute;
mod contract;
mod emit;
pub mod geometry;
mod manifest;
pub mod temporal;

pub use manifest::write_bundle;
use naga::{ShaderStage, back::pipeline_constants, valid};
use sha2::{Digest, Sha256};

/// Versioned against the actual wgpu-hal layout and coordinate conventions.
pub const CONTRACT: &str = "incant-output-v1;naga=29.0.4;wgpu=29.0.4;hal=e99f5305ded96ff7006f0714d043a7f735bd45c2;msl=2.4;spv=1.0,no-y-flip;hlsl=6.0;bounds=restrict;loops=bounded;layout=texture0,uniform80";

#[derive(Debug, thiserror::Error)]
pub enum CookError {
    #[error("output shader parse failed: {0}")]
    Parse(String),
    #[error("output shader validation failed: {0}")]
    Validate(String),
    #[error("unsupported output shader contract: {0}")]
    Contract(&'static str),
    #[error("output shader emission failed: {0}")]
    Emit(String),
    #[error("output artifact write failed: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, CookError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Metal,
    Vulkan,
    Dx12,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Vertex,
    Fragment,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub backend: Backend,
    pub stage: Stage,
    pub entry: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub source_hash: [u8; 32],
    pub producer_hash: [u8; 32],
    pub artifacts: Vec<Artifact>,
}

/// Cook only engine-owned source. Callers must not expose the resulting trusted
/// passthrough objects to project-controlled shader inputs.
pub fn cook_output(source: &str) -> Result<Bundle> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|e| CookError::Parse(e.emit_to_string(source)))?;
    let info = valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::empty())
        .validate(&module)
        .map_err(|e| CookError::Validate(e.to_string()))?;
    contract::validate(&module, &info)?;
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
                emit::emit(&compact, &compact_info, &module, stage, name, backend)?;
            artifacts.push(Artifact {
                backend,
                stage: tag,
                entry,
                bytes,
            });
        }
    }
    let producer = [
        include_str!("lib.rs"),
        include_str!("contract.rs"),
        include_str!("emit.rs"),
        include_str!("manifest.rs"),
        include_str!("../Cargo.toml"),
    ]
    .join("\n");
    Ok(Bundle {
        producer_hash: Sha256::digest(producer).into(),
        source_hash: Sha256::digest(source.as_bytes()).into(),
        artifacts,
    })
}
