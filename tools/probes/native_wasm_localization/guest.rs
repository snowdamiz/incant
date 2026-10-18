//! Ordinary authored Rust guest: copied coverage, committed locales and explicit state.
#![no_std]
use incant_module::localization::{
    LocaleSettings, LocaleSnapshot, LocalizeRequest, MissingKind, ReportRequest, ReportSnapshot,
};
use incant_module::{EntityId, Error, IDENTITY, Key, LogLevel, Result, Tick, persistent};

incant_module::persistent_state!(schema = [41; 32], bytes = 8);
incant_module::module!(simulate);

const LOCALES: [&str; 4] = ["en", "fr-CA", "de", "zh-Hant"];
const TABLES: [&str; 2] = ["00000000000000000000000004", "00000000000000000000000005"];

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

fn ensure(value: bool) -> Result<()> {
    if value { Ok(()) } else { Err(Error::Malformed) }
}

fn total(phase: usize) -> usize {
    [0, 130, 132, 64][phase]
}

fn locale(snapshot: LocaleSnapshot<'_>, phase: usize) -> Result<()> {
    ensure(snapshot.locale == LOCALES[phase] && snapshot.pseudo == (phase == 2))?;
    let mut fallbacks = snapshot.fallbacks();
    ensure(fallbacks.next() == if phase == 2 { Some("fr") } else { None })?;
    ensure(fallbacks.next().is_none())
}

fn page(report: ReportSnapshot<'_>, offset: usize, phase: usize, hash: &mut u32) -> Result<()> {
    ensure(report.total as usize == total(phase))?;
    for (index, row) in report.entries().enumerate() {
        let ordinal = index + offset;
        let per_table = [1, 65, 66, 32][phase];
        ensure(ordinal < total(phase))?;
        let local = ordinal % per_table;
        let empty = phase == 2 && local == 0;
        let n = if phase == 3 {
            local * 2 + 1
        } else {
            local - usize::from(phase == 2 && !empty)
        };
        let key = [
            b'k',
            b'0' + (n / 100) as u8,
            b'0' + ((n / 10) % 10) as u8,
            b'0' + (n % 10) as u8,
        ];
        let expected_key: &[u8] = if empty { b"empty" } else { &key };
        ensure(
            row.table_id == TABLES[ordinal / per_table]
                && row.key.as_bytes() == expected_key
                && row.requested_locale == LOCALES[phase]
                && row.resolved_locale
                    == Some(if phase != 3 && (empty || n % 2 == 0) {
                        "fr"
                    } else {
                        "en"
                    })
                && row.kind == MissingKind::Translation,
        )?;
        for text in [
            row.table_id,
            row.key,
            row.requested_locale,
            row.resolved_locale.ok_or(Error::Malformed)?,
            "translation",
        ] {
            for byte in text.bytes().chain(core::iter::once(0)) {
                *hash = (*hash ^ u32::from(byte)).wrapping_mul(16777619);
            }
        }
    }
    Ok(())
}

fn simulate(t: &mut Tick<'_>, _: f64) -> Result<()> {
    let mut state = [0; 8];
    t.read_persistent(0, &mut state)?;
    let count = persistent::read_u64(&state, 0)?
        .checked_add(1)
        .ok_or(Error::Bounds)?;
    ensure((1..=12).contains(&count))?;
    persistent::write_u64(&mut state, 0, count)?;
    t.write_persistent(0, &state)?;
    let phase = (count as usize - 1) % 4;
    let mut settings_bytes = [0; 256];
    let current = t.locale(&mut settings_bytes)?;
    locale(current, phase)?;
    let mut scratch = [0; 16384];
    let next = (phase + 1) % 4;
    t.set_locale(
        LocaleSettings {
            locale: LOCALES[next],
            fallbacks: if next == 2 { &["fr"] } else { &[] },
            pseudo: next == 2,
        },
        &mut scratch,
    )?;
    t.log(current.locale, LogLevel::Info)?;
    // Tentative numeric, explicit-state, locale and log writes precede refusal.
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let mut ids = [EntityId([0; 16]); 4];
        let ids = t.read_ids(chunk, &mut ids)?;
        let mut storage = [IDENTITY; 4];
        let rows = t.read_transforms(chunk, &mut storage)?;
        for (id, row) in ids.iter().zip(rows.iter_mut()) {
            if *id == EntityId::from_u128(1) {
                row[0] = count as f64;
            }
        }
        t.write_transforms(chunk, rows)?;
    }
    let mut input_bytes = [0; 2048];
    let keys = t.input(&mut input_bytes)?.keyboard;
    for (key, request) in [
        (
            Key::Digit1,
            ReportRequest {
                offset: 4097,
                limit: 1,
            },
        ),
        (
            Key::Digit2,
            ReportRequest {
                offset: 0,
                limit: 0,
            },
        ),
        (
            Key::Digit3,
            ReportRequest {
                offset: 0,
                limit: 129,
            },
        ),
    ] {
        if keys.held(key as u8) {
            let _ = t.localization_report(request, &mut scratch);
            return Ok(());
        }
    }
    if keys.held(Key::Digit4 as u8) {
        let _ = t.localization_report(ReportRequest::default(), &mut scratch[..1]);
        return Ok(());
    }
    if keys.held(Key::Digit5 as u8) {
        for _ in 0..65 {
            let _ = t.localization_report(
                ReportRequest {
                    offset: 4096,
                    limit: 1,
                },
                &mut scratch,
            );
        }
        return Ok(());
    }
    if keys.held(Key::Digit6 as u8) {
        return Err(Error::Malformed);
    }
    let mut saved = [0; 16384];
    let first = t.localization_report(ReportRequest::default(), &mut saved)?;
    let rest = t.localization_report(
        ReportRequest {
            offset: 128,
            limit: 128,
        },
        &mut scratch,
    )?;
    ensure(first.len() == total(phase).min(128) && first.len() + rest.len() == total(phase))?;
    let mut hash = 2166136261;
    // Replaced host reply storage must not change the first copied page.
    page(first, 0, phase, &mut hash)?;
    let prefix_hash = hash;
    page(rest, 128, phase, &mut hash)?;
    for (offset, limit) in [(0, 1), (1, 127), (4096, 128)] {
        let part = t.localization_report(ReportRequest { offset, limit }, &mut scratch)?;
        ensure(
            part.len()
                == total(phase)
                    .saturating_sub(offset as usize)
                    .min(limit as usize),
        )?;
        page(part, offset as usize, phase, &mut 0)?;
    }
    let mut request = [0; 256];
    let missing = t.localize(
        LocalizeRequest {
            table_id: TABLES[0],
            key: "absent",
            arguments: &[],
        },
        &mut request,
        &mut scratch,
    )?;
    ensure(
        missing.requested_locale == current.locale
            && missing.resolved_locale.is_none()
            && missing.missing.is_some_and(|m| {
                m.table_id == TABLES[0] && m.key == "absent" && m.kind == MissingKind::Key
            }),
    )?;
    let after = t.localization_report(ReportRequest::default(), &mut scratch)?;
    ensure(after.len() == total(phase).min(128))?;
    let mut after_hash = 2166136261;
    page(after, 0, phase, &mut after_hash)?;
    ensure(after_hash == prefix_hash)?;
    locale(t.locale(&mut scratch)?, phase)?;
    for index in 0..t.chunk_count()? {
        let chunk = t.chunk(index)?;
        let mut ids = [EntityId([0; 16]); 4];
        let ids = t.read_ids(chunk, &mut ids)?;
        let mut storage = [IDENTITY; 4];
        let rows = t.read_transforms(chunk, &mut storage)?;
        for (id, row) in ids.iter().zip(rows.iter_mut()) {
            if *id == EntityId::from_u128(1) {
                row[..3].copy_from_slice(&[count as f64, total(phase) as f64, f64::from(hash)]);
            }
        }
        t.write_transforms(chunk, rows)?;
    }
    Ok(())
}
