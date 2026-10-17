use super::*;
pub(super) fn decode(
    sections: codec::Sections<'_>,
    code: &CompiledScript,
    world: &NativeWorld,
) -> Result<ScriptState> {
    #[cfg(any(feature = "wasmtime-host", feature = "static-module"))]
    if sections.wasm.is_some() {
        return Err(CheckpointError::ProgramMismatch);
    }
    let active = codec::active(sections.active)?;
    let store = codec::store(sections.store)?;
    let mut schedule = codec::schedule(sections.schedule)?;
    schedule.coroutines = codec::coroutines(sections.coroutines, schedule.tick)?;
    if schedule.tick != world.tick() {
        return Err(CheckpointError::Invalid("world/script tick pairing"));
    }
    let scope_known = |scope| match (&code.bindings, scope) {
        (None, Scope::Global) => true,
        (Some(b), Scope::Source(id)) => b.sources.binary_search(&id).is_ok(),
        _ => false,
    };
    match &code.bindings {
        Some(b) => {
            if b.scene_sha256 != world.source_scene_fingerprint()
                || active.iter().any(|id| {
                    b.instances.binary_search_by_key(id, |i| i.entity).is_err()
                        || world.entity_identity(*id).is_none()
                })
            {
                return Err(CheckpointError::Invalid("active attachment catalog"));
            }
        }
        None if !active.is_empty() => {
            return Err(CheckpointError::Invalid("active attachments"));
        }
        None => {}
    }
    for key in store.entries.keys() {
        if !scope_known(key.scope) {
            return Err(CheckpointError::Invalid("persistent source scope"));
        }
        if let Some(id) = key.entity {
            let b = code
                .bindings
                .as_ref()
                .ok_or(CheckpointError::Invalid("persistent instance"))?;
            let i = b
                .instances
                .binary_search_by_key(&id, |i| i.entity)
                .map_err(|_| CheckpointError::Invalid("persistent instance"))?;
            if key.scope != Scope::Source(b.instances[i].source)
                || active.binary_search(&id).is_err()
            {
                return Err(CheckpointError::Invalid("persistent instance lifetime"));
            }
        }
    }
    validate_tasks(code.bindings.as_deref(), &active, &schedule)?;
    if schedule.timers.keys().any(|k| !scope_known(k.scope)) {
        return Err(CheckpointError::Invalid("timer source scope"));
    }
    Ok(ScriptState {
        active,
        store,
        schedule,
    })
}
