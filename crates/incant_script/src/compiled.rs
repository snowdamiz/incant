use crate::{ScriptError, diagnostics, vm};
use rquickjs::{Ctx, Function, Module, WriteOptions};
use std::{sync::Arc, time::Duration};

/// Bytecode produced by this exact pinned QuickJS compiler. The bytes are private
/// and immutable: there is deliberately no safe loader for arbitrary cache/file
/// bytes. A content hash alone would not make untrusted QuickJS bytecode safe.
/// File export and authenticated artifact loading remain separate work.
#[derive(Clone)]
pub struct CompiledScript {
    bytes: Arc<[u8]>,
}

impl CompiledScript {
    /// Compile SWC CommonJS output without executing project code. This belongs
    /// at authoring/cook time. Runtime ticks execute only the compiled module.
    pub fn compile(source: &str) -> Result<Self, ScriptError> {
        if source.len() > 1_000_000 {
            return Err(ScriptError::InputLimit);
        }
        let (_runtime, context, deadline) = vm(Duration::from_millis(50))?;
        let wrapper = format!(
            r#"
            const defineBehavior = value => value;
            const behavior = (function(exports) {{ "use strict";
                {source}
                ; return exports.default;
            }})({{}});
            if (!behavior || typeof behavior.update !== 'function') throw new Error('default behavior.update required');
            if ('initialState' in behavior || 'onTimer' in behavior) throw new Error('state and timers are not migrated to the native bulk host');
            export function update(chunks, dt) {{ return behavior.update({{chunks, dt}}); }}
        "#
        );
        let bytes = context.with(|ctx| {
            let module = Module::declare(ctx.clone(), "incant-native-behavior", wrapper)
                .map_err(|error| diagnostics::javascript(&ctx, error, "compilation", &deadline))?;
            module
                .write(WriteOptions::default())
                .map_err(|error| diagnostics::javascript(&ctx, error, "compilation", &deadline))
        })?;
        diagnostics::check_deadline(&deadline)?;
        Ok(Self {
            bytes: bytes.into(),
        })
    }

    pub fn byte_len(&self) -> usize {
        self.bytes.len()
    }

    pub(crate) fn instantiate<'js>(&self, ctx: &Ctx<'js>) -> rquickjs::Result<Function<'js>> {
        // SAFETY: bytes can only be made by compile() above, using the identical
        // pinned engine and build. No externally provided bytecode is accepted.
        // ScriptHost retains this Arc allocation until after VM destruction.
        let module = unsafe { Module::load(ctx.clone(), &self.bytes) }?;
        let (module, promise) = module.eval()?;
        promise.result::<()>().ok_or(rquickjs::Error::Unknown)??;
        module.get("update")
    }
}
