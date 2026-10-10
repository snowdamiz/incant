//! Cook-only source parsing and generated native embedding code.
use crate::{CompiledScript, ScriptError, ScriptMetadata, artifact::*, diagnostics, vm};
use rquickjs::{Module, WriteOptions};
use std::time::Duration;

impl CompiledScript {
    pub fn compile(source: &str) -> Result<Self, ScriptError> {
        Self::compile_named("incant-native-behavior", source)
    }

    /// Compile bounded SWC CommonJS output without executing project code.
    /// Source text is stripped; bytecode line tables and the module name remain.
    pub fn compile_named(name: &str, source: &str) -> Result<Self, ScriptError> {
        if source.len() > MAX_SOURCE_BYTES || !valid_name(name) {
            return Err(ScriptError::InputLimit);
        }
        let (_runtime, context, deadline) = vm(Duration::from_millis(50), true)?;
        let wrapper = format!(
            "const defineBehavior = value => value;\n\
             const behavior = (function(exports) {{\n\
             \"use strict\";\n\
             {source}\n\
             ; return exports.default;\n\
             }})({{}});\n\
             if (!behavior || typeof behavior.update !== 'function') throw new Error('default behavior.update required');\n\
             if ('initialState' in behavior || 'onTimer' in behavior) throw new Error('state and timers are not migrated to the native bulk host');\n\
             export function update(chunks, dt, commands, input) {{ return behavior.update({{chunks, dt, commands, input}}); }}\n"
        );
        let (bytes, hardening) = context.with(|ctx| {
            let compile = |name: &str, text: &str| {
                Module::declare(ctx.clone(), name, text)
                    .and_then(|module| {
                        module.write(WriteOptions {
                            strip_source: true,
                            ..WriteOptions::default()
                        })
                    })
                    .map_err(|error| diagnostics::javascript(&ctx, error, "compilation", &deadline))
            };
            Ok::<_, ScriptError>((
                compile(name, &wrapper)?,
                compile("incant-native-hardening", include_str!("harden.js"))?,
            ))
        })?;
        diagnostics::check_deadline(&deadline)?;
        let metadata = ScriptMetadata {
            format: ARTIFACT_FORMAT,
            engine: ENGINE_VERSION.into(),
            build_id: BUILD_ID.into(),
            target: TARGET.into(),
            source_name: name.into(),
            source_sha256: digest(source.as_bytes()),
            wrapper_prefix_lines: WRAPPER_PREFIX_LINES,
            bytecode_sha256: digest(&bytes),
            hardening_sha256: digest(&hardening),
        };
        validate(&metadata, &bytes, &hardening)?;
        Ok(Self {
            bytes: bytes.into(),
            hardening: hardening.into(),
            metadata,
            allow_source: true,
        })
    }

    /// Generate a Rust module directly from this compiler result, never from a
    /// bytecode cache. All project-controlled data is escaped as Rust literals.
    /// Only a trusted native build may include the returned module.
    pub fn embedded_rust(&self) -> String {
        let m = &self.metadata;
        format!(
            "// Generated from trusted cook output. Do not replace with project caches.\n\
             pub fn load() -> Result<incant_script::CompiledScript, incant_script::ScriptError> {{\n\
             let artifact = incant_script::EmbeddedScript {{\n\
             metadata: incant_script::ScriptMetadata {{\n\
             format: {}, engine: {:?}.into(), build_id: {:?}.into(), target: {:?}.into(),\n\
             source_name: {:?}.into(), source_sha256: {:?}, wrapper_prefix_lines: {},\n\
             bytecode_sha256: {:?}, hardening_sha256: {:?},\n\
             }}, bytecode: &{:?}, hardening: &{:?},\n\
             }};\n\
             // SAFETY: this module was emitted directly from the trusted compiler\n\
             // in this native build, not read from a project executable cache.\n\
             unsafe {{ incant_script::CompiledScript::from_embedded(artifact) }}\n\
             }}\n",
            m.format,
            m.engine,
            m.build_id,
            m.target,
            m.source_name,
            m.source_sha256,
            m.wrapper_prefix_lines,
            m.bytecode_sha256,
            m.hardening_sha256,
            self.bytes,
            self.hardening,
        )
    }
}
