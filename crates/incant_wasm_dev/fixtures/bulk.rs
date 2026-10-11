//! Trusted bulk sample using the public generated-contract Rust SDK.
#![no_std]
use incant_module::{Result, Tick, Transform};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(run);
fn run(tick: &mut Tick<'_>, dt: f64) -> Result<()> {
    let mut values: [Transform; 1024] = [[0.; 10]; 1024];
    for index in 0..tick.chunk_count()? {
        let chunk = tick.chunk(index)?;
        let rows = tick.read_transforms(chunk, &mut values)?;
        for row in rows.iter_mut() {
            row[0] += dt;
        }
        tick.write_transforms(chunk, rows)?;
    }
    Ok(())
}
