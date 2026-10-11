//! Trusted compiled fixture, not a generated SDK or an untrusted build service.
#![no_std]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
#[link(wasm_import_module = "incant_v1")]
unsafe extern "C" {
    fn chunk_count(epoch: i64) -> i32;
    fn chunk_len(epoch: i64, chunk: i32) -> i32;
    fn chunk_flags(epoch: i64, chunk: i32) -> i32;
    fn read_velocities(epoch: i64, chunk: i32, destination: *mut f64);
    fn write_velocities(epoch: i64, chunk: i32, source: *const f64);
    fn read_ids(epoch: i64, chunk: i32, destination: *mut u8);
    fn read_transforms(epoch: i64, chunk: i32, destination: *mut f64);
    fn write_transforms(epoch: i64, chunk: i32, source: *const f64);
    fn spawn(epoch: i64, id: *const u8, parent: i32, transform: *const f64, velocity: *const f64);
}
#[link(wasm_import_module = "incant_input_v1")]
unsafe extern "C" {
    fn snapshot_len(epoch: i64) -> i32;
    fn read_snapshot(epoch: i64, destination: *mut u8, capacity: i32) -> i32;
}
static mut INPUT: [u8; 131072] = [0; 131072];
static mut SPEEDS: [f64; 96] = [0.; 96];
static mut SPAWNED: bool = false;
// This fixture reads only the named-action section. It checks the version,
// bounds and tags before using it; raw controls remain available in the ABI.
fn jump(bytes: &[u8]) -> (bool, bool) {
    if bytes.len() < 28 || &bytes[..8] != b"INPINP01" { core::arch::wasm32::unreachable(); }
    let u32_at = |at| u32::from_le_bytes(bytes[at..at+4].try_into().unwrap());
    if u32_at(8) as usize != bytes.len() || u32_at(24) > 64 { core::arch::wasm32::unreachable(); }
    let mut at=28;
    for _ in 0..u32_at(24) {
        if at+20 > bytes.len() { core::arch::wasm32::unreachable(); }
        let len=usize::from(bytes[at]); let flags=bytes[at+2];
        if len>64 || at+20+len>bytes.len() || bytes[at+1]>2 || flags & !7 != 0 || bytes[at+3]!=0 { core::arch::wasm32::unreachable(); }
        if &bytes[at+20..at+20+len]==b"jump" {
            if bytes[at+1]!=0 { core::arch::wasm32::unreachable(); }
            return (flags & 1 != 0, flags & 2 != 0);
        }
        at+=20+len;
    }
    (false,false)
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
        let len=snapshot_len(epoch);
        if !(28..=131072).contains(&len) { core::arch::wasm32::unreachable(); }
        let input=(&raw mut INPUT).cast::<u8>();
        if read_snapshot(epoch,input,131072)!=len { core::arch::wasm32::unreachable(); }
        let (active,pressed)=jump(core::slice::from_raw_parts(input,len as usize));
        for chunk in 0..chunk_count(epoch) {
            let count = chunk_len(epoch, chunk);
            if !(0..=32).contains(&count) { core::arch::wasm32::unreachable(); }
            let values = (&raw mut VALUES).cast::<f64>();
            let ids = (&raw mut IDS).cast::<u8>();
            read_ids(epoch, chunk, ids);
            read_transforms(epoch, chunk, values);
            let speeds=(&raw mut SPEEDS).cast::<f64>();
            let has_velocity=chunk_flags(epoch,chunk) & 1 != 0;
            if has_velocity { read_velocities(epoch,chunk,speeds); }
            for row in 0..count as usize {
                let id = core::slice::from_raw_parts(ids.add(row * 16), 16);
                if id[..15] == [0; 15] && id[15] == 1 {
                    if pressed { *values.add(row * 10) += 4.; }
                    if has_velocity { *speeds.add(row*3)=if active {2.} else {0.}; }
                    *values.add(row * 10 + 1) = f64::from(TICKS) * dt;
                }
            }
            write_transforms(epoch, chunk, values);
            if has_velocity { write_velocities(epoch,chunk,speeds); }
        }
        if pressed && !SPAWNED {
            SPAWNED=true;
            let mut id = [0u8; 16]; id[15] = 3;
            let transform = [0., 0., 0., 0., 0., 0., 1., 1., 1., 1.];
            let velocity = [2., 0., 0.];
            spawn(epoch, id.as_ptr(), -1, transform.as_ptr(), velocity.as_ptr());
        }
    }
}
