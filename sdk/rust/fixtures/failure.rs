//! Adversarial SDK callback controls, never an application template.
#![no_std]
use incant_module::{EntityId, Error, IDENTITY, LogLevel};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
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
        tick.log("staged", LogLevel::Warn)?;
        let chunk = tick.chunk(0)?;
        let mut values = [IDENTITY; 8];
        let rows = tick.read_transforms(chunk, &mut values)?;
        rows[0][0] = 42. + old as f64;
        tick.write_transforms(chunk, rows)?;
        if old == 0 {
            tick.pause(&EntityId::from_u128(1))?;
            tick.spawn(&EntityId::from_u128(88), None, &IDENTITY, None)?;
        } else {
            tick.restart(&EntityId::from_u128(1))?;
            tick.despawn(&EntityId::from_u128(88))?;
        }
        if cfg!(sdk_expired) && old != 0 {
            incant_module::__private::run(old, dt, |stale, _| {
                stale.chunk_count()?;
                Ok(())
            });
        }
        if cfg!(sdk_log_bounds) {
            // Valid UTF-8 primitive whose byte length exceeds the message cap.
            let bytes = [b'x'; 1025];
            let _ignored = tick.log(core::str::from_utf8(&bytes).unwrap(), LogLevel::Info);
        }
        if cfg!(sdk_log_invalid) {
            let _ignored = tick.log("bad\r", LogLevel::Info);
        }
        if cfg!(sdk_capacity) {
            let _ignored = tick.read_ids(chunk, &mut []);
        }
        if cfg!(sdk_callback_error) {
            return Err(Error::Callback);
        }
        if cfg!(sdk_malformed) {
            let bytes = [0; 24];
            incant_module::InputSnapshot::parse(&bytes)?;
        }
        Ok(())
    });
}
