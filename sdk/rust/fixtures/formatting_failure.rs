//! Ignored formatting errors must reject every staged category.
#![no_std]
use incant_module::{EntityId, IDENTITY, LogLevel, Result, Tick, Error};
use incant_module::localization::{CalendarDate, DateLength, LocaleSettings};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut scratch = [0; 1024];
    tick.set_locale(LocaleSettings { locale: "fr", fallbacks: &[], pseudo: true }, &mut scratch)?;
    tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
    let chunk = tick.chunk(0)?; let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?; rows[0][0] = 42.; tick.write_transforms(chunk, rows)?;
    tick.log("staged", LogLevel::Warn)?;
    if cfg!(sdk_format_capacity) { let _ignored = tick.format_number(1., &mut []); }
    if cfg!(sdk_format_number) { let _ignored = tick.format_number(f64::NAN, &mut scratch); }
    if cfg!(sdk_format_range) { let _ignored = tick.format_number(9_007_199_254_740_992., &mut scratch); }
    if cfg!(sdk_format_date) { let _ignored = tick.format_date(CalendarDate { year: 1900, month: 2, day: 29 }, DateLength::Medium, &mut scratch); }
    if cfg!(sdk_format_year) { let _ignored = tick.format_date(CalendarDate { year: 10000, month: 1, day: 1 }, DateLength::Medium, &mut scratch); }
    if cfg!(sdk_format_quota) { for _ in 0..65 { let _ignored = tick.format_number(1., &mut scratch); } }
    if cfg!(sdk_format_callback) { return Err(Error::Callback); }
    // Even successful work cannot follow an ignored local argument/capacity error.
    let _ignored = tick.format_number(2., &mut scratch);
    Ok(())
}
