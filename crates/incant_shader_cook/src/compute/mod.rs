//! Fixed, engine-owned compute program. No project shader input is exposed.
mod contract;
mod emit;
mod spirv_workgroup;
mod static_workgroup;
use crate::{Backend, CookError, Result};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, fs, path::Path};

pub const CONTRACT: &str = "incant-cluster-v1;naga=29.0.4;wgpu=29.0.4;group64x1x1;shared=atomic4+128x4;msl=2.4,static-threadgroup,sizes4;spv=1.0,implicit-workgroup-layout;hlsl=6.0,grid-b0,light-t0,counts-u0,masks-u1,sizes-b1,special-b2;bounds=restrict;loops=bounded";
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub backend: Backend,
    pub entry: String,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    pub source_hash: [u8; 32],
    pub producer_hash: [u8; 32],
    pub artifacts: Vec<Artifact>,
}

pub fn cook(source: &str) -> Result<Bundle> {
    let m = naga::front::wgsl::parse_str(source)
        .map_err(|e| CookError::Parse(e.emit_to_string(source)))?;
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&m)
    .map_err(|e| CookError::Validate(e.to_string()))?;
    contract::validate(&m, &info)?;
    let artifacts = [Backend::Metal, Backend::Vulkan, Backend::Dx12]
        .into_iter()
        .map(|backend| {
            let (bytes, entry) = emit::emit(&m, &info, backend)?;
            Ok(Artifact {
                backend,
                entry,
                bytes,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let producer = [
        include_str!("mod.rs"),
        include_str!("contract.rs"),
        include_str!("emit.rs"),
        include_str!("static_workgroup.rs"),
        include_str!("spirv_workgroup.rs"),
        include_str!("../../Cargo.toml"),
    ]
    .join("\n");
    Ok(Bundle {
        source_hash: Sha256::digest(source).into(),
        producer_hash: Sha256::digest(producer).into(),
        artifacts,
    })
}
pub fn write_bundle(bundle: &Bundle, directory: &Path) -> Result<()> {
    if bundle.artifacts.len() != 3 {
        return Err(CookError::Contract("cluster artifact count"));
    }
    fs::create_dir_all(directory)?;
    let mut generated = format!(
        "const MANIFEST_CONTRACT: &str = {CONTRACT:?};\nconst SOURCE_HASH: [u8;32] = {:?};\nconst PRODUCER_HASH: [u8;32] = {:?};\nconst ARTIFACTS: [Artifact;3] = [\n",
        bundle.source_hash, bundle.producer_hash
    );
    for a in &bundle.artifacts {
        let file = format!("{:?}-Compute.shader", a.backend);
        fs::write(directory.join(&file), &a.bytes)?;
        let hash: [u8; 32] = Sha256::digest(&a.bytes).into();
        writeln!(generated,"Artifact {{ backend: wgpu::Backend::{:?}, stage: Stage::Compute, entry: {:?}, hash: {:?}, bytes: include_bytes!({:?}) }},",a.backend,a.entry,hash,file).expect("writing String");
    }
    generated.push_str("];\n");
    fs::write(directory.join("manifest.rs"), generated)?;
    Ok(())
}
