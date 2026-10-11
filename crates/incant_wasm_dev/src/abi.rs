//! Wasmtime adapter for the shared incant_v1 capability. No mutation rules here.
use incant_wasm_runtime::{Call, Capability};
use wasmtime::{Caller, Linker, Result, StoreLimits};
pub struct State {
    pub limits: StoreLimits,
    pub capability: Capability,
}
fn invoke(mut caller: Caller<'_, State>, epoch: i64, call: Call) -> Result<i32> {
    let memory = caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or_else(|| wasmtime::format_err!("missing memory"))?;
    let (bytes, state) = memory.data_and_store_mut(&mut caller);
    Ok(state.capability.invoke(bytes, epoch, call)?)
}
pub fn install(linker: &mut Linker<State>) -> Result<()> {
    linker.func_wrap(
        "incant_v1",
        "chunk_count",
        |c: Caller<'_, State>, e: i64| invoke(c, e, Call::ChunkCount),
    )?;
    linker.func_wrap(
        "incant_v1",
        "chunk_len",
        |c: Caller<'_, State>, e: i64, i: i32| invoke(c, e, Call::ChunkLen(i)),
    )?;
    linker.func_wrap(
        "incant_v1",
        "chunk_flags",
        |c: Caller<'_, State>, e: i64, i: i32| invoke(c, e, Call::ChunkFlags(i)),
    )?;
    linker.func_wrap(
        "incant_v1",
        "read_ids",
        |c: Caller<'_, State>, e: i64, i: i32, p: i32| {
            invoke(c, e, Call::ReadIds(i, p)).map(|_| ())
        },
    )?;
    for (name, velocity, write) in [
        ("read_transforms", false, false),
        ("write_transforms", false, true),
        ("read_velocities", true, false),
        ("write_velocities", true, true),
    ] {
        linker.func_wrap(
            "incant_v1",
            name,
            move |c: Caller<'_, State>, e: i64, chunk: i32, ptr: i32| {
                invoke(
                    c,
                    e,
                    Call::Column {
                        chunk,
                        ptr,
                        velocity,
                        write,
                    },
                )
                .map(|_| ())
            },
        )?;
    }
    linker.func_wrap(
        "incant_v1",
        "spawn",
        |c: Caller<'_, State>, e: i64, id: i32, parent: i32, transform: i32, velocity: i32| {
            invoke(
                c,
                e,
                Call::Spawn {
                    id,
                    parent,
                    transform,
                    velocity,
                },
            )
            .map(|_| ())
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "despawn",
        |c: Caller<'_, State>, e: i64, id: i32| invoke(c, e, Call::Despawn(id)).map(|_| ()),
    )?;
    linker.func_wrap(
        "incant_v1",
        "set_velocity",
        |c: Caller<'_, State>, e: i64, id: i32, ptr: i32| {
            invoke(c, e, Call::SetVelocity(id, ptr)).map(|_| ())
        },
    )?;
    linker.func_wrap(
        "incant_v1",
        "set_parent",
        |c: Caller<'_, State>, e: i64, id: i32, ptr: i32| {
            invoke(c, e, Call::SetParent(id, ptr)).map(|_| ())
        },
    )?;
    Ok(())
}
