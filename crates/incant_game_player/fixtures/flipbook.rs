//! Trusted source-free game fixture for the additive input + flipbook ABI.
//! Not an arbitrary-source compiler service or a complete generated Rust SDK.
#![no_std]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
#[link(wasm_import_module = "incant_v1")]
unsafe extern "C" {
    fn chunk_count(epoch: i64) -> i32;
    fn chunk_len(epoch: i64, chunk: i32) -> i32;
    fn chunk_flags(epoch: i64, chunk: i32) -> i32;
    fn read_velocities(epoch: i64, chunk: i32, ptr: *mut f64);
    fn write_velocities(epoch: i64, chunk: i32, ptr: *const f64);
}
#[link(wasm_import_module = "incant_input_v1")]
unsafe extern "C" {
    #[link_name = "snapshot_len"]
    fn input_len(epoch: i64) -> i32;
    #[link_name = "read_snapshot"]
    fn read_input(epoch: i64, ptr: *mut u8, capacity: i32) -> i32;
}
#[link(wasm_import_module = "incant_flipbook_v1")]
unsafe extern "C" {
    fn snapshot_len(epoch: i64) -> i32;
    fn read_snapshot(epoch: i64, ptr: *mut u8, capacity: i32) -> i32;
    fn control(epoch: i64, id: *const u8, kind: i32, frame: i32);
}
const MAX: usize = 16 + 32768 * 24;
static mut INPUT: [u8; 131072] = [0; 131072];
static mut SNAPSHOT: [u8; MAX] = [0; MAX];
static mut AFTER: [u8; MAX] = [0; MAX];
static mut SPEEDS: [f64; 96] = [0.; 96];
// Only this fixture's named actions are needed; raw input remains in the ABI.
fn actions(bytes: &[u8]) -> u32 {
    if bytes.len() < 28 || &bytes[..8] != b"INPINP01" {
        core::arch::wasm32::unreachable();
    }
    let u32_at = |at| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    if u32_at(8) as usize != bytes.len() || u32_at(24) > 64 {
        core::arch::wasm32::unreachable();
    }
    let mut at = 28;
    let mut bits = 0;
    for _ in 0..u32_at(24) {
        if at + 20 > bytes.len() {
            core::arch::wasm32::unreachable();
        }
        let len = usize::from(bytes[at]);
        let flags = bytes[at + 2];
        if len > 64
            || at + 20 + len > bytes.len()
            || bytes[at + 1] > 2
            || flags & !7 != 0
            || bytes[at + 3] != 0
        {
            core::arch::wasm32::unreachable();
        }
        let name = &bytes[at + 20..at + 20 + len];
        if name == b"right" && flags & 1 != 0 {
            bits |= 1;
        }
        if flags & 2 != 0 {
            bits |= match name {
                b"flip_pause" => 2,
                b"flip_seek" => 4,
                b"flip_play" => 8,
                b"flip_restart" => 16,
                b"flip_invalid" => 32,
                b"flip_trap" => 64,
                _ => 0,
            };
        }
        at += 20 + len;
    }
    bits
}
#[unsafe(no_mangle)]
pub extern "C" fn incant_abi() -> i32 {
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn update(epoch: i64, _dt: f64) {
    unsafe {
        let length = input_len(epoch);
        if !(28..=131072).contains(&length) {
            core::arch::wasm32::unreachable();
        }
        let input = (&raw mut INPUT).cast::<u8>();
        if read_input(epoch, input, 131072) != length {
            core::arch::wasm32::unreachable();
        }
        let bits = actions(core::slice::from_raw_parts(input, length as usize));
        for chunk in 0..chunk_count(epoch) {
            if chunk_flags(epoch, chunk) & 1 == 0 {
                continue;
            }
            let count = chunk_len(epoch, chunk);
            if !(0..=32).contains(&count) {
                core::arch::wasm32::unreachable();
            }
            let speeds = (&raw mut SPEEDS).cast::<f64>();
            read_velocities(epoch, chunk, speeds);
            for i in 0..count as usize {
                *speeds.add(i * 3) = if bits & 1 != 0 { 3. } else { 0. };
            }
            write_velocities(epoch, chunk, speeds);
        }
        let length = snapshot_len(epoch);
        if !(16..=MAX as i32).contains(&length) {
            core::arch::wasm32::unreachable();
        }
        let snapshot = (&raw mut SNAPSHOT).cast::<u8>();
        if read_snapshot(epoch, snapshot, MAX as i32) != length {
            core::arch::wasm32::unreachable();
        }
        let rows = core::slice::from_raw_parts(snapshot, length as usize);
        let count = u32::from_le_bytes(rows[8..12].try_into().unwrap()) as usize;
        if &rows[..8] != b"INCFB001"
            || rows[12..16] != 24u32.to_le_bytes()
            || count > 32768
            || rows.len() != 16 + count * 24
        {
            core::arch::wasm32::unreachable();
        }
        for row in rows[16..].chunks_exact(24) {
            let frame = u32::from_le_bytes(row[16..20].try_into().unwrap());
            let flags = u32::from_le_bytes(row[20..24].try_into().unwrap());
            if frame >= 256 || flags & !3 != 0 {
                core::arch::wasm32::unreachable();
            }
            if row[..15] != [0; 15] || row[15] != 1 {
                continue;
            }
            for (bit, kind, frame) in [(2, 1, 0), (4, 3, 2), (8, 0, 0), (16, 2, 0), (32, 3, 3)] {
                if bits & bit != 0 {
                    control(epoch, row.as_ptr(), kind, frame);
                }
            }
        }
        let after = (&raw mut AFTER).cast::<u8>();
        if read_snapshot(epoch, after, MAX as i32) != length {
            core::arch::wasm32::unreachable();
        }
        if rows != core::slice::from_raw_parts(after, length as usize) {
            core::arch::wasm32::unreachable();
        }
        if bits & 64 != 0 {
            core::arch::wasm32::unreachable();
        }
    }
}
