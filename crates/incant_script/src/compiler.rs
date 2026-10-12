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
        Self::compile_inner(name, source, None)
    }

    /// Compile the source-keyed output of the trusted union TypeScript cook.
    /// Initial attachments/props are paired with the canonical binary scene.
    pub fn compile_project(
        name: &str,
        source: &str,
        bindings: crate::ScriptBindings,
    ) -> Result<Self, ScriptError> {
        Self::compile_inner(name, source, Some(bindings))
    }

    fn compile_inner(
        name: &str,
        source: &str,
        bindings: Option<crate::ScriptBindings>,
    ) -> Result<Self, ScriptError> {
        if source.len() > MAX_SOURCE_BYTES || !valid_name(name) {
            return Err(ScriptError::InputLimit);
        }
        let (_runtime, context, deadline) = vm(Duration::from_millis(50), true)?;
        let dispatch = if let Some(bindings) = &bindings {
            let ids = bindings
                .sources()
                .iter()
                .map(|id| format!("{:?}", ulid::Ulid::from_bytes(id.0).to_string()))
                .collect::<Vec<_>>()
                .join(",");
            let entries = bindings
                .sources()
                .iter()
                .enumerate()
                .map(|(index, id)| {
                    format!(
                        "entry({index}, {:?})",
                        ulid::Ulid::from_bytes(id.0).to_string()
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            let persistent = bindings
                .sources()
                .iter()
                .enumerate()
                .map(|(i, _)| format!("opted(updates[{i}])"))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "const expected = [{ids}];\n\
                if (!behavior || !nativeIsArray(behavior.sources) || behavior.sources.length !== expected.length) throw new Error('native source catalog mismatch');\n\
                function entry(i, id) {{ const item = behavior.sources[i]; if (!item || item.id !== id) throw new Error('native source identity mismatch'); const value = item.behavior; check(value); return value; }}\n\
                const updates = [{entries}];\n\
                export const persistent = [{persistent}];\n\
                export function update(chunks, dt, commands, input, physics, navigation, localization, clock, timers, log, flipbooks, animations, group, instances, state, hud) {{ return updates[group].update({{chunks, dt, commands, input, physics, navigation, localization, clock, timers, log, flipbooks, animations, instances, state, hud}}); }}\n"
            )
        } else {
            "check(behavior);\nexport const persistent = [opted(behavior)];\nexport function update(chunks, dt, commands, input, physics, navigation, localization, clock, timers, log, flipbooks, animations, group, instances, state, hud) { return behavior.update({chunks, dt, commands, input, physics, navigation, localization, clock, timers, log, flipbooks, animations, instances, state, hud}); }\n".into()
        };
        let wrapper = format!(
            "const defineBehavior = value => value; const nativeIsArray = Array.isArray;\n\
             const behavior = (function(exports) {{\n\
             \"use strict\";\n\
             {source}\n\
             ; return exports.default;\n\
             }})({{}});\n\
             function check(value) {{ if (!value || typeof value.update !== 'function') throw new Error('default behavior.update required');\n\
             if ('initialState' in value || 'onTimer' in value) throw new Error('legacy initialState/onTimer are unsupported; handle native timers.events in update'); }}\n\
             function opted(value) {{ const setting = value.persistence; if (setting === undefined) return false; if (!setting || setting.version !== 1) throw new Error('persistence version 1 required'); return true; }}\n\
             {dispatch}"
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
            bindings_sha256: bindings.as_ref().map(crate::ScriptBindings::fingerprint),
        };
        validate(&metadata, &bytes, &hardening)?;
        Ok(Self {
            bytes: bytes.into(),
            hardening: hardening.into(),
            metadata,
            // Authored gameplay executes bytecode only. Source parsing belongs
            // to this fresh cook VM; eval/Function stay absent during updates.
            allow_source: bindings.is_none(),
            bindings: bindings.map(std::sync::Arc::new),
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
             bytecode_sha256: {:?}, hardening_sha256: {:?}, bindings_sha256: {:?},\n\
             }}, bytecode: &{:?}, hardening: &{:?}, bindings: {},\n\
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
            m.bindings_sha256,
            self.bytes,
            self.hardening,
            self.bindings.as_ref().map_or_else(
                || "None".into(),
                |bindings| format!("Some({})", crate::bindings_embed::rust(bindings))
            ),
        )
    }
}
