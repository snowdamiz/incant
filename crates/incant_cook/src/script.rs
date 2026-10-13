//! Trusted local source compilation. Supplied executable caches are never read.
use incant_script::{ScriptBuild, ScriptError, ScriptMetadata, artifact};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const MAX_SOURCE_MAP_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ScriptCookError {
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[error("script cook target is not admitted for compiler host {host}; requested {target}")]
    CrossTarget { host: &'static str, target: String },
    #[error("script source map is oversized or is not a supported version-3 map")]
    SourceMap,
}

/// The generated Rust goes into a fresh build OUT_DIR; symbol metadata/map stay
/// outside the player. Do not accept these outputs from an untrusted project.
pub struct CookedScriptArtifact {
    pub rust_module: String,
    pub symbol_manifest: String,
    pub source_map: Option<String>,
}

/// Compile bounded CommonJS emitted by the pinned SWC authoring step. No project
/// code executes. Cross-target cooks require the explicit pinned bytecode ABI
/// route; this does not enable authored-project cross-target packaging.
pub fn cook_script(
    name: &str,
    commonjs: &str,
    source_map: Option<&str>,
    target: &str,
) -> Result<CookedScriptArtifact, ScriptCookError> {
    if !artifact::compatibility::admits(artifact::TARGET, target, artifact::BytecodeAbi::current())
    {
        return Err(ScriptCookError::CrossTarget {
            host: artifact::TARGET,
            target: target.into(),
        });
    }
    if let Some(map) = source_map {
        if map.len() > MAX_SOURCE_MAP_BYTES {
            return Err(ScriptCookError::SourceMap);
        }
        let value: Value = serde_json::from_str(map).map_err(|_| ScriptCookError::SourceMap)?;
        if value.get("version").and_then(Value::as_u64) != Some(3)
            || !value
                .get("sources")
                .is_some_and(|s| s.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
            || !value
                .get("names")
                .is_some_and(|s| s.as_array().is_some_and(|a| a.iter().all(Value::is_string)))
            || !value.get("mappings").is_some_and(Value::is_string)
            || value.get("sections").is_some()
        {
            return Err(ScriptCookError::SourceMap);
        }
    }
    let compiled = ScriptBuild::compile(name, commonjs, target)?;
    Ok(artifact_from_parts(
        compiled.metadata(),
        compiled.byte_len(),
        compiled.embedded_rust(),
        source_map,
    ))
}

/// The program has already been freshly compiled by this trusted producer.
/// Project source maps originate in the fixed, hash-checked Node compiler.
#[cfg(feature = "typescript")]
pub(crate) fn artifact_from_build(
    compiled: &ScriptBuild,
    source_map: Option<&str>,
) -> CookedScriptArtifact {
    artifact_from_parts(
        compiled.metadata(),
        compiled.byte_len(),
        compiled.embedded_rust(),
        source_map,
    )
}

fn artifact_from_parts(
    m: &ScriptMetadata,
    byte_len: usize,
    rust_module: String,
    source_map: Option<&str>,
) -> CookedScriptArtifact {
    let hex = |bytes: &[u8]| bytes.iter().map(|v| format!("{v:02x}")).collect::<String>();
    let manifest = json!({
        "format": m.format, "engine": m.engine, "build_id": m.build_id,
        "target": m.target, "producer_target": m.producer_target, "bytecode_abi": m.bytecode_abi,
        "source_name": m.source_name,
        "source_sha256": hex(&m.source_sha256),
        "bytecode_sha256": hex(&m.bytecode_sha256),
        "bindings_sha256": m.bindings_sha256.as_ref().map(|v| hex(v)),
        "hardening_sha256": hex(&m.hardening_sha256),
        "bytecode_bytes": byte_len,
        "wrapper_prefix_lines": m.wrapper_prefix_lines,
        "source_map_sha256": source_map.map(|map| hex(&Sha256::digest(map.as_bytes()))),
        "debug_line_tables": true, "embedded_source": false,
        "trust": "fresh compiler output embedded by trusted native build; hashes are not authentication",
        "symbolication": "map CommonJS locations after subtracting wrapper_prefix_lines; automatic runtime stack remapping remains unsupported",
    });
    CookedScriptArtifact {
        rust_module,
        symbol_manifest: format!("{manifest:#}\n"),
        source_map: source_map.map(str::to_owned),
    }
}
