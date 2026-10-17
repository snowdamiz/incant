#![no_std]
use incant_module::{EntityId,IDENTITY,Result,Tick,coroutines::{Task,Next},persistent::{read_u64,write_u64}};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::persistent_state!(schema = [31;32], bytes = 8);
incant_module::coroutine_module!(simulate; "walk" => ([32;32], 8, walk));
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
    Ok(if count < 3 { Next::Wait(2) } else { Next::Done })
}
