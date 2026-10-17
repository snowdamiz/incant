//! Explicit counter data survives restore; this module's ordinary globals do not.
#![no_std]
use core::sync::atomic::{AtomicU32, Ordering};
use incant_module::{EntityId, Error, IDENTITY, LogLevel, Result, Tick, persistent};
static CALLS: AtomicU32 = AtomicU32::new(0);
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::persistent_state!(schema = [5; 32], bytes = 16);
incant_module::module!(simulate);
fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut state = [0u8; 16];
    t.read_persistent(0, &mut state)?;
    let count = persistent::read_u64(&state, 0)?
        .checked_add(1)
        .filter(|value| *value <= u64::from(u32::MAX))
        .ok_or(Error::Bounds)?;
    persistent::write_u64(&mut state, 0, count)?;
    t.write_persistent(0, &state)?;
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let mut ids = [EntityId([0; 16]); 16];
        let ids = t.read_ids(chunk, &mut ids)?;
        let mut storage = [IDENTITY; 16];
        let rows = t.read_transforms(chunk, &mut storage)?;
        for (id, row) in ids.iter().zip(rows.iter_mut()) {
            if *id == EntityId::from_u128(1) {
                row[0] = count as f64;
            }
        }
        t.write_transforms(chunk, rows)?;
    }
    let calls = CALLS.fetch_add(1, Ordering::Relaxed);
    t.log(
        if calls == 0 {
            "fresh-guest"
        } else {
            "continued-guest"
        },
        LogLevel::Info,
    )?;
    if cfg!(state_restart) {
        t.request_restart()?;
    }
    if cfg!(state_owner) {
        t.despawn(&EntityId::from_u128(999))?;
    }
    if cfg!(state_callback) {
        return Err(Error::Callback);
    }
    if cfg!(state_quota) {
        for _ in 0..256 {
            let _ = t.write_persistent(0, &state);
        }
    }
    Ok(())
}
