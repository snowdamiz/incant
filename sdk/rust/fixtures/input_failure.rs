//! Actual input-operation failures must discard previously staged writes/logs.
#![no_std]
use incant_module::{Error, IDENTITY, INPUT_MAX_BYTES, LogLevel};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
static mut INPUT: [u8; INPUT_MAX_BYTES] = [0; INPUT_MAX_BYTES];
static mut OLD: i64 = 0;
#[unsafe(no_mangle)]
pub extern "C" fn incant_abi() -> i32 {
    incant_module::__private::ABI_VERSION
}
#[unsafe(no_mangle)]
pub extern "C" fn update(epoch: i64, dt: f64) {
    let old = unsafe {
        let previous = OLD;
        OLD = epoch;
        previous
    };
    incant_module::__private::run(epoch, dt, |tick, _| {
        let chunk = tick.chunk(0)?;
        let mut transforms = [IDENTITY; 8];
        let rows = tick.read_transforms(chunk, &mut transforms)?;
        rows[0][0] += 42.;
        tick.write_transforms(chunk, rows)?;
        tick.log("input staged", LogLevel::Warn)?;
        if cfg!(input_scratch) {
            let _ignored = tick.input(&mut []);
        } else {
            let snapshot = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
            if !snapshot.focused {
                return Err(Error::Callback);
            }
        }
        if cfg!(input_stale) && old != 0 {
            incant_module::__private::run(old, dt, |stale, _| {
                let _ignored = stale.input(unsafe { &mut *(&raw mut INPUT) });
                Ok(())
            });
        }
        if cfg!(input_callback) {
            return Err(Error::Callback);
        }
        Ok(())
    });
}
