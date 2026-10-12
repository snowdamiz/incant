//! Trusted standalone ICU formatting; compile offline with tools/rust_guest.py.
#![no_std]
use incant_module::{EntityId, IDENTITY, LogLevel, Result, Tick, Error};
use incant_module::localization::{CalendarDate, DateLength, LocaleSettings};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut scratch = [0; 2048]; let mut locale_scratch = [0; 256];
    let locale = tick.locale(&mut locale_scratch)?;
    tick.log(locale.locale, LogLevel::Debug)?;
    for value in [-1234.5, 9_007_199_254_740_991., -9_007_199_254_740_991., -0.] {
        let text = tick.format_number(value, &mut scratch)?;
        tick.log(text, LogLevel::Info)?;
    }
    for date in [CalendarDate { year: 2024, month: 2, day: 29 }, CalendarDate { year: 2000, month: 2, day: 29 }, CalendarDate { year: -9999, month: 1, day: 1 }] {
        for length in [DateLength::Short, DateLength::Medium, DateLength::Long] {
            let text = tick.format_date(date, length, &mut scratch)?;
            tick.log(text, LogLevel::Info)?;
        }
    }
    let next = match locale.locale { "en" => "fr", "fr" => "ar", "ar" => "de-CH", _ => "en" };
    tick.set_locale(LocaleSettings { locale: next, fallbacks: &[], pseudo: true }, &mut scratch)?;
    let before = tick.format_number(-1234.5, &mut scratch)?;
    tick.log(before, LogLevel::Info)?;
    if tick.locale(&mut scratch)?.locale != locale.locale { return Err(Error::Malformed); }
    let chunk = tick.chunk(0)?; let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?; rows[0][0] += 1.; tick.write_transforms(chunk, rows)?;
    if locale.locale == "en" { tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?; }
    Ok(())
}
