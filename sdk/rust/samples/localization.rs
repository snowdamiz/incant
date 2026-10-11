//! Trusted no_std example: committed-locale queries, rich message arguments and
//! atomic locale changes. Build with tools/rust_guest.py, never at runtime.
#![no_std]
use incant_module::{EntityId, IDENTITY, LogLevel, Result, Tick};
use incant_module::localization::*;
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
const TABLE: &str = "00000000000000000000000002";
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut request = [0; 2048]; let mut result = [0; 2048]; let mut locale_bytes = [0; 256];
    let locale = tick.locale(&mut locale_bytes)?;
    tick.log(locale.locale, LogLevel::Debug)?;
    let args = [Argument { name: "name", value: MessageArgument::Text("雪🙂") }, Argument { name: "n", value: MessageArgument::Number(21.) }, Argument { name: "gender", value: MessageArgument::Text("female") }, Argument { name: "date", value: MessageArgument::Date(CalendarDate { year: 2024, month: 2, day: 29 }) }];
    for key in ["hello", "items", "party", "rank", "dated", "absent"] {
        let text = tick.localize(LocalizeRequest { table_id: TABLE, key, arguments: &args }, &mut request, &mut result)?;
        if text.requested_locale != locale.locale || (key == "absent" && !matches!(text.missing, Some(MissingString { kind: MissingKind::Key, .. }))) { return Err(incant_module::Error::Malformed); }
        tick.log(text.text, LogLevel::Info)?;
    }
    let settings = match locale.locale {
        "en" => LocaleSettings { locale: "fr", fallbacks: &[], pseudo: false },
        "fr" => LocaleSettings { locale: "ru", fallbacks: &[], pseudo: false },
        "ru" => LocaleSettings { locale: "de-CH", fallbacks: &["zh-Hant", "en"], pseudo: true },
        _ => LocaleSettings { locale: "en", fallbacks: &[], pseudo: false },
    };
    tick.set_locale(settings, &mut request)?;
    // A queued change cannot alter this callback's observation.
    if tick.locale(&mut result)?.locale != locale.locale { return Err(incant_module::Error::Malformed); }
    let chunk = tick.chunk(0)?; let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?; rows[0][0] += 1.; tick.write_transforms(chunk, rows)?;
    if locale.locale == "en" { tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?; }
    Ok(())
}
