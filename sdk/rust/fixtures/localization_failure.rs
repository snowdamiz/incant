//! Ignored guest-method errors and explicit callback failure must reject prefixes.
#![no_std]
use incant_module::{EntityId, IDENTITY, LogLevel, Result, Tick};
use incant_module::localization::*;
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut request = [0; 1024]; let mut result = [0; 1024];
    tick.set_locale(LocaleSettings { locale: "fr", fallbacks: &[], pseudo: false }, &mut request)?;
    tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
    let chunk = tick.chunk(0)?; let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?; rows[0][0] = 42.; tick.write_transforms(chunk, rows)?;
    tick.log("staged", LogLevel::Warn)?;
    if cfg!(sdk_locale_capacity) { let _ignored = tick.locale(&mut []); }
    if cfg!(sdk_locale_number) {
        let _ignored = tick.localize(LocalizeRequest { table_id: "00000000000000000000000002", key: "items", arguments: &[Argument { name: "n", value: MessageArgument::Number(f64::NAN) }] }, &mut request, &mut result);
    }
    if cfg!(sdk_locale_duplicate) { let _ignored = tick.set_locale(LocaleSettings { locale: "en", fallbacks: &["fr", "fr"], pseudo: false }, &mut request); }
    if cfg!(sdk_locale_callback) { return Err(incant_module::Error::Callback); }
    Ok(())
}
