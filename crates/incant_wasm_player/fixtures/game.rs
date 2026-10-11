//! Trusted compiled fixture, not a generated SDK or an untrusted build service.
#![no_std]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
#[link(wasm_import_module = "incant_v1")]
unsafe extern "C" {
    fn chunk_count(epoch: i64) -> i32;
    fn chunk_len(epoch: i64, chunk: i32) -> i32;
    fn read_ids(epoch: i64, chunk: i32, destination: *mut u8);
    fn read_transforms(epoch: i64, chunk: i32, destination: *mut f64);
    fn write_transforms(epoch: i64, chunk: i32, source: *const f64);
    fn spawn(epoch: i64, id: *const u8, parent: i32, transform: *const f64, velocity: *const f64);
}
static mut VALUES: [f64; 320] = [0.; 320];
static mut IDS: [u8; 512] = [0; 512];
static mut TICKS: u32 = 0;
#[unsafe(no_mangle)]
pub extern "C" fn incant_abi() -> i32 { 1 }
#[unsafe(no_mangle)]
pub extern "C" fn update(epoch: i64, dt: f64) {
    // StableId bytes are big-endian; pointers only address bounded guest memory.
    unsafe {
        TICKS += 1;
        for chunk in 0..chunk_count(epoch) {
            let count = chunk_len(epoch, chunk);
            if !(0..=32).contains(&count) { core::arch::wasm32::unreachable(); }
            let values = (&raw mut VALUES).cast::<f64>();
            let ids = (&raw mut IDS).cast::<u8>();
            read_ids(epoch, chunk, ids);
            read_transforms(epoch, chunk, values);
            for row in 0..count as usize {
                let id = core::slice::from_raw_parts(ids.add(row * 16), 16);
                if id[..15] == [0; 15] && id[15] == 1 {
                    if TICKS == 1 { *values.add(row * 10) += 4.; }
                    *values.add(row * 10 + 1) = f64::from(TICKS) * dt;
                }
            }
            write_transforms(epoch, chunk, values);
        }
        if TICKS == 1 {
            let mut id = [0u8; 16]; id[15] = 3;
            let transform = [0., 0., 0., 0., 0., 0., 1., 1., 1., 1.];
            let velocity = [2., 0., 0.];
            spawn(epoch, id.as_ptr(), -1, transform.as_ptr(), velocity.as_ptr());
        }
    }
}
