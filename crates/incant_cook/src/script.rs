//! Trusted local source compilation. Supplied executable caches are never read.
use incant_script::{CompiledScript, ScriptError, artifact};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const MAX_SOURCE_MAP_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ScriptCookError {
    #[error(transparent)]
    Script(#[from] ScriptError),
    #[error("script cook target must equal compiler host {host}; requested {target}")]
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
/// code executes. The target must be this compiler's native target; cross-target
/// export needs a separate compatibility proof and fails explicitly here.
pub fn cook_script(
    name: &str,
    commonjs: &str,
    source_map: Option<&str>,
    target: &str,
) -> Result<CookedScriptArtifact, ScriptCookError> {
    if target != artifact::TARGET {
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
    let compiled = CompiledScript::compile_named(name, commonjs)?;
    let m = compiled.metadata();
    let hex = |bytes: &[u8]| bytes.iter().map(|v| format!("{v:02x}")).collect::<String>();
    let manifest = json!({
        "format": m.format, "engine": m.engine, "build_id": m.build_id,
        "target": m.target, "source_name": m.source_name,
        "source_sha256": hex(&m.source_sha256),
        "bytecode_sha256": hex(&m.bytecode_sha256),
        "hardening_sha256": hex(&m.hardening_sha256),
        "bytecode_bytes": compiled.byte_len(),
        "wrapper_prefix_lines": m.wrapper_prefix_lines,
        "source_map_sha256": source_map.map(|map| hex(&Sha256::digest(map.as_bytes()))),
        "debug_line_tables": true, "embedded_source": false,
        "trust": "fresh compiler output embedded by trusted native build; hashes are not authentication",
        "symbolication": "map CommonJS locations after subtracting wrapper_prefix_lines; automatic runtime stack remapping remains unsupported",
    });
    Ok(CookedScriptArtifact {
        rust_module: compiled.embedded_rust(),
        symbol_manifest: format!("{manifest:#}\n"),
        source_map: source_map.map(str::to_owned),
    })
}
