//! Named fixed-tick timers and explicit durable module state, without wall time.
#![no_std]
use incant_module::{Error, IDENTITY, Result, Tick, persistent};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::persistent_state!(schema = [18; 32], bytes = 16);
incant_module::module!(simulate);
fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut scratch = [0; incant_module::TIMER_MAX_BYTES];
    let timers = t.timers(&mut scratch)?;
    let tick = timers.tick();
    let elapsed = timers.elapsed_seconds();
    let mut state = [0; 16];
    t.read_persistent(0, &mut state)?;
    let mut count = persistent::read_u64(&state, 0)?;
    let mut order = persistent::read_u64(&state, 8)?;
    for (id, due) in timers.events() {
        if due != tick {
            return Err(Error::Malformed);
        }
        count += 1;
        order = order
            .wrapping_mul(31)
            .wrapping_add(u64::from(id.as_bytes()[0]));
    }
    persistent::write_u64(&mut state, 0, count)?;
    persistent::write_u64(&mut state, 8, order)?;
    t.write_persistent(0, &state)?;
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let mut storage = [IDENTITY; 16];
        let rows = t.read_transforms(chunk, &mut storage)?;
        for row in rows.iter_mut() {
            row[0] = tick as f64;
            row[1] = count as f64;
            row[2] = elapsed;
            row[7] = order as f64 + 1.;
        }
        t.write_transforms(chunk, rows)?;
    }
    match tick {
        1 => {
            t.set_timer("z", 2, Some(2))?;
            t.set_timer("a", 10, None)?;
            t.set_timer("a", 2, None)?;
            t.set_timer("removed", 1, None)?;
            t.cancel_timer("removed")?;
            t.set_timer("far", u32::MAX, None)?;
        }
        3 => {
            t.cancel_timer("z")?;
            t.set_timer("a", 1, None)?;
        }
        4 => t.set_timer("p", 2, Some(2))?,
        _ => {}
    }
    // Deliberately ignored SDK errors must still reject the entire transaction.
    if cfg!(timer_scratch) {
        let _ = t.timers(&mut []);
    }
    if cfg!(timer_delay) {
        let _ = t.set_timer("bad", 0, None);
    }
    if cfg!(timer_interval) {
        let _ = t.set_timer("bad", 1, Some(0));
    }
    if cfg!(timer_id) {
        let _ = t.cancel_timer("not allowed");
    }
    if cfg!(timer_actions) {
        for _ in 0..257 {
            let _ = t.cancel_timer("absent");
        }
    }
    if cfg!(timer_pending) {
        for index in 0u16..129 {
            let bytes = [b't', b'A' + (index / 26) as u8, b'A' + (index % 26) as u8];
            let id = core::str::from_utf8(&bytes).map_err(|_| Error::Malformed)?;
            let _ = t.set_timer(id, 10, None);
        }
    }
    if cfg!(timer_owner) {
        t.despawn(&incant_module::EntityId::from_u128(999))?;
    }
    if cfg!(timer_callback) {
        return Err(Error::Callback);
    }
    if cfg!(timer_trap) {
        core::arch::wasm32::unreachable();
    }
    Ok(())
}
