#![no_std]
use incant_module::{EntityId,IDENTITY,Result,Tick,coroutines::{Task,Next},persistent::{read_u64,write_u64}};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::persistent_state!(schema = [31;32], bytes = 8);
incant_module::coroutine_module!(simulate; "walk" => ([if cfg!(coroutine_schema) {33} else {32};32], 8, walk));
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut timer = [0;incant_module::TIMER_MAX_BYTES];
    let clock = tick.timers(&mut timer)?.tick();
    let mut scratch = [0;incant_module::coroutines::COROUTINE_MAX_BYTES];
    if clock == 1 {
        tick.start_coroutine("z","walk",&0u64.to_le_bytes(),Some(EntityId::from_u128(1)),&mut scratch)?;
        tick.start_coroutine("a","walk",&0u64.to_le_bytes(),None,&mut scratch)?;
        tick.start_coroutine("removed","walk",&0u64.to_le_bytes(),None,&mut scratch)?;
    }
    if clock == 2 { tick.cancel_coroutine("removed")?; }
    tick.write_persistent(0,&clock.to_le_bytes())?;
    // A rejected command remains fatal even if application code ignores it.
    if cfg!(coroutine_unknown) { let _ = tick.start_coroutine("bad","missing",&[0;8],None,&mut scratch); }
    if cfg!(coroutine_duplicate) { let _ = tick.start_coroutine("a","walk",&[0;8],None,&mut scratch); }
    if cfg!(coroutine_scratch) { let _ = tick.start_coroutine("bad","walk",&[0;8],None,&mut []); }
    if cfg!(coroutine_owner) { let _ = tick.start_coroutine("bad","walk",&[0;8],Some(EntityId::from_u128(999)),&mut scratch); }
    if cfg!(coroutine_frame) { let _ = tick.start_coroutine("bad","walk",&[0;7],None,&mut scratch); }
    if cfg!(coroutine_actions) { for _ in 0..257 { let _ = tick.cancel_coroutine("absent"); } }
    if cfg!(coroutine_tasks) {
        for index in 0u16..129 {
            let id = [b't', b'A' + (index / 26) as u8, b'A' + (index % 26) as u8];
            let id = core::str::from_utf8(&id).map_err(|_| incant_module::Error::Malformed)?;
            let _ = tick.start_coroutine(id,"walk",&[0;8],None,&mut scratch);
        }
    }
    Ok(())
}
fn walk(tick:&mut Tick<'_>, task:Task<'_>) -> Result<Next> {
    let count = read_u64(task.state,0)? + 1;
    write_u64(task.state,0,count)?;
    let mut scratch = [IDENTITY;16];
    for index in 0..tick.chunk_count()? {
        let chunk = tick.chunk(index)?;
        let rows = tick.read_transforms(chunk,&mut scratch)?;
        for row in rows.iter_mut() { row[0] = row[0]*10. + if task.id == "a" {1.} else {2.};row[1] += 1.; }
        tick.write_transforms(chunk,rows)?;
    }
    if cfg!(coroutine_wait) { return Ok(Next::Wait(0)); }
    if cfg!(coroutine_callback) { return Err(incant_module::Error::Callback); }
    if cfg!(coroutine_trap) { core::arch::wasm32::unreachable(); }
    Ok(if count < 3 { Next::Wait(2) } else { Next::Done })
}
