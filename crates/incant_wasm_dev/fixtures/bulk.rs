//! Trusted repository fixture compiled directly by the pinned rustc. This is
//! not the future generated module SDK or an untrusted Rust build service.
#![no_std]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
#[link(wasm_import_module = "incant_v1")]
unsafe extern "C" {
    fn chunk_count(epoch: i64) -> i32;
    fn chunk_len(epoch: i64, chunk: i32) -> i32;
    fn read_transforms(epoch: i64, chunk: i32, destination: *mut f64);
    fn write_transforms(epoch: i64, chunk: i32, source: *const f64);
}
static mut VALUES: [f64; 10240] = [0.; 10240];
#[unsafe(no_mangle)]
pub extern "C" fn incant_abi() -> i32 { 1 }
#[unsafe(no_mangle)]
pub extern "C" fn update(epoch: i64, dt: f64) {
    // Guest pointers are bounded linear-memory offsets, never native pointers.
    // The synchronous host cannot reenter this module; buffer has 1024 rows.
    unsafe {
        for chunk in 0..chunk_count(epoch) {
            let count = chunk_len(epoch, chunk);
            if !(0..=1024).contains(&count) { core::arch::wasm32::unreachable(); }
            let values = (&raw mut VALUES).cast::<f64>();
            read_transforms(epoch, chunk, values);
            for row in 0..count as usize { *values.add(row * 10) += dt; }
            write_transforms(epoch, chunk, values);
        }
    }
}
