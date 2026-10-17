//! Restart requests are deferred metadata; component writes still commit normally.
#![no_std]
use core::sync::atomic::{AtomicU32, Ordering};
use incant_module::{EntityId, Error, IDENTITY, Result, Tick};
static CALLS: AtomicU32 = AtomicU32::new(0);
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let calls = CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let mut storage = [IDENTITY; 8];
        let rows = t.read_transforms(chunk, &mut storage)?;
        for row in rows.iter_mut() {
            if row[0] != f64::from(calls - 1) {
                return Err(Error::Malformed);
            }
            row[0] += 1.;
        }
        t.write_transforms(chunk, rows)?;
    }
    if !cfg!(restart_once) || calls == 1 {
        t.request_restart()?;
        t.request_restart()?;
    }
    if cfg!(restart_quota) {
        for _ in 0..1024 {
            let _ = t.request_restart();
        }
    }
    if cfg!(restart_owner) {
        t.despawn(&EntityId::from_u128(999))?;
    }
    if cfg!(restart_callback) {
        return Err(Error::Malformed);
    }
    if cfg!(restart_trap) {
        core::arch::wasm32::unreachable();
    }
    Ok(())
}
