use crate::{EmbeddedScript, ScriptError, ScriptMetadata, artifact};
use rquickjs::{Ctx, Function, Module};
use std::sync::Arc;

/// Private immutable compiler-produced bytecode, retained through VM destruction.
/// No safe loader accepts arbitrary project, cache, mod or network bytes.
#[derive(Clone)]
pub struct CompiledScript {
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) hardening: Arc<[u8]>,
    pub(crate) metadata: ScriptMetadata,
    pub(crate) allow_source: bool,
    pub(crate) bindings: Option<Arc<crate::ScriptBindings>>,
}

impl CompiledScript {
    /// Cross the native build's executable-code trust boundary.
    ///
    /// # Safety
    /// The caller must establish that both blobs were freshly produced by the
    /// trusted Incant cook for this exact engine/build/target and remain its
    /// unmodified output. This is intended only for cook-generated Rust included
    /// in a trusted executable build. The hardening blob must be the engine's
    /// fixed hardening module, never project code. A matching hash/metadata does
    /// **not** establish these facts. Never call this on supplied executable
    /// caches, project/mod/network bytes, or generated Rust from such inputs.
    ///
    /// ```compile_fail
    /// fn arbitrary_bytes(artifact: incant_script::EmbeddedScript) {
    ///     incant_script::CompiledScript::from_embedded(artifact);
    /// }
    /// ```
    pub unsafe fn from_embedded(artifact: EmbeddedScript) -> Result<Self, ScriptError> {
        artifact::validate(&artifact.metadata, artifact.bytecode, artifact.hardening)?;
        if artifact
            .bindings
            .as_ref()
            .map(crate::ScriptBindings::fingerprint)
            != artifact.metadata.bindings_sha256
        {
            return Err(ScriptError::ArtifactIntegrity);
        }
        Ok(Self {
            bytes: artifact.bytecode.into(),
            hardening: artifact.hardening.into(),
            metadata: artifact.metadata,
            allow_source: false,
            bindings: artifact.bindings.map(Arc::new),
        })
    }

    pub fn byte_len(&self) -> usize {
        self.bytes.len() + self.hardening.len()
    }

    pub fn metadata(&self) -> &ScriptMetadata {
        &self.metadata
    }

    pub(crate) fn harden<'js>(
        &self,
        ctx: &Ctx<'js>,
    ) -> rquickjs::Result<(Function<'js>, Function<'js>)> {
        // SAFETY: compiler or the documented native-build trust crossing above
        // is the sole source. The fixed engine module precedes all guest code.
        let module = unsafe { Module::load(ctx.clone(), &self.hardening) }?;
        let (module, promise) = module.eval()?;
        promise.result::<()>().ok_or(rquickjs::Error::Unknown)??;
        Ok((module.get("freeze")?, module.get("persistentWrapper")?))
    }

    pub(crate) fn instantiate<'js>(
        &self,
        ctx: &Ctx<'js>,
    ) -> rquickjs::Result<(Function<'js>, Vec<bool>, crate::coroutines::Catalog)> {
        // SAFETY: only trusted compiler-produced bytes enter this allocation.
        // The host retains both Arc allocations until after VM destruction.
        let module = unsafe { Module::load(ctx.clone(), &self.bytes) }?;
        let (module, promise) = module.eval()?;
        promise.result::<()>().ok_or(rquickjs::Error::Unknown)??;
        let flags: rquickjs::Array = module.get("persistent")?;
        let count = self.bindings.as_ref().map_or(1, |b| b.sources().len());
        if flags.len() != count {
            return Err(rquickjs::Error::Unknown);
        }
        let flags = flags.iter::<bool>().collect::<rquickjs::Result<Vec<_>>>()?;
        let rows: rquickjs::Array = module.get("routines")?;
        if rows.len() != count {
            return Err(rquickjs::Error::Unknown);
        }
        let mut catalog = crate::coroutines::Catalog::default();
        let mut total = 0;
        for row in rows.iter::<rquickjs::Array>() {
            let row = row?;
            total += row.len();
            if row.len() > 32 || total > 128 {
                return Err(rquickjs::Error::Unknown);
            }
            let mut names = Vec::new();
            for name in row.iter::<rquickjs::Value>() {
                let name = crate::persistence::value::text(ctx, &name?, 64)?;
                if !crate::coroutines::valid_id(&name) || names.contains(&name) {
                    return Err(rquickjs::Error::Unknown);
                }
                names.push(name);
            }
            catalog.0.push(names.into());
        }
        catalog.1 = total != 0;
        Ok((module.get("update")?, flags, catalog))
    }
}
