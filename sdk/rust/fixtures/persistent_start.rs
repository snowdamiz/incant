//! Startup has no active state capability, including ignored import failures.
#![no_std]
incant_module::persistent_state!(schema = [5; 32], bytes = 16);
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
#[link(wasm_import_module = "incant_state_v1")]
unsafe extern "C" {
    fn write(epoch: i64, ptr: i32, offset: i32, len: i32) -> i32;
}
#[unsafe(no_mangle)]
pub extern "C" fn incant_abi() -> i32 {
    let bytes = [9u8; 16];
    // SAFETY: the initialized borrowed bytes stay live for this synchronous copy.
    // Epoch zero is intentionally unauthorized and the enclosing host must fail.
    let _ = unsafe { write(0, bytes.as_ptr() as usize as i32, 0, 16) };
    1
}
#[unsafe(no_mangle)]
pub extern "C" fn update(_: i64, _: f64) {}
