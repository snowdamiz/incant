//! Prepared coverage pages share the callback's committed localization snapshot.
#![no_std]
use incant_module::{Error, IDENTITY, LogLevel, Result, Tick};
use incant_module::localization::{LocaleSettings, ReportRequest};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut scratch = [0; 4096];
    let mut saved = [0; 4096];
    let chunk = tick.chunk(0)?;
    let mut transforms = [IDENTITY; 16];
    let rows = tick.read_transforms(chunk, &mut transforms)?;
    rows[0][0] += 1.;
    tick.write_transforms(chunk, rows)?;
    let mut locale_bytes = [0; 256];
    let locale = tick.locale(&mut locale_bytes)?;
    let next = match locale.locale { "en" => "fr", "fr" => "ar", "ar" => "zh-Hant", _ => "en" };
    tick.set_locale(LocaleSettings { locale: next, fallbacks: &[], pseudo: true }, &mut scratch)?;
    tick.log(locale.locale, LogLevel::Info)?;
    if cfg!(report_offset) {
        let _ = tick.localization_report(ReportRequest { offset: 4097, limit: 1 }, &mut scratch);
        return Ok(());
    }
    if cfg!(report_limit) {
        let _ = tick.localization_report(ReportRequest { offset: 0, limit: 129 }, &mut scratch);
        return Ok(());
    }
    if cfg!(report_capacity) {
        let _ = tick.localization_report(ReportRequest::default(), &mut scratch[..1]);
        return Ok(());
    }
    if cfg!(report_quota) {
        for _ in 0..64 { let _ = tick.localization_report(ReportRequest::default(), &mut scratch); }
        return Ok(());
    }
    let first = tick.localization_report(ReportRequest { offset: 0, limit: 1 }, &mut saved)?;
    let total = first.total;
    tick.log(match total { 0 => "0", 3 => "3", 4 => "4", _ => return Err(Error::Malformed) }, LogLevel::Info)?;
    let rest = tick.localization_report(ReportRequest { offset: 1, limit: 128 }, &mut scratch)?;
    if rest.total != total || first.len() + rest.len() != total as usize { return Err(Error::Malformed); }
    // Both frames remain readable after the host reply slot has been replaced.
    for row in first.entries().chain(rest.entries()) {
        tick.log(row.key, LogLevel::Info)?;
        tick.log(row.requested_locale, LogLevel::Info)?;
        tick.log(row.resolved_locale.unwrap_or("none"), LogLevel::Info)?;
    }
    let end = tick.localization_report(ReportRequest { offset: 4096, limit: 128 }, &mut scratch)?;
    if !end.is_empty() || end.total != total { return Err(Error::Malformed); }
    if tick.locale(&mut scratch)?.locale != locale.locale { return Err(Error::Malformed); }
    if cfg!(report_callback) { return Err(Error::Malformed); }
    Ok(())
}
