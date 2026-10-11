//! Wasmtime adapter for the shared incant_v1 capability. No mutation rules here.
use incant_wasm_runtime::{Call, Capability};
use wasmtime::{Caller, Result, StoreLimits};
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
#[path = "generated.rs"]
mod generated;
pub use generated::install;
