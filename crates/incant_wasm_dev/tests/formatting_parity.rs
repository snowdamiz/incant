#![cfg(feature = "static-conformance")]
#[path = "../../incant_runtime/tests/support/localization.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn bytes(name: &str) -> Vec<u8> {
    if name.starts_with("rust-formatting") {
        std::fs::read(
            std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")),
        )
        .unwrap()
    } else {
        wat::parse_file(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../incant_wasm_runtime/fixtures")
                .join(format!("{name}.wat")),
        )
        .unwrap()
    }
}
fn expected(locale: &str) -> Vec<String> {
    let mut values = vec![locale.to_owned()];
    for value in [
        -1234.5,
        9_007_199_254_740_991.,
        -9_007_199_254_740_991.,
        -0.,
    ] {
        values.push(incant_localization::format_number(locale, value).unwrap());
    }
    for date in [
        CalendarDate {
            year: 2024,
            month: 2,
            day: 29,
        },
        CalendarDate {
            year: 2000,
            month: 2,
            day: 29,
        },
        CalendarDate {
            year: -9999,
            month: 1,
            day: 1,
        },
    ] {
        for length in [DateLength::Short, DateLength::Medium, DateLength::Long] {
            values.push(incant_localization::format_date(locale, &date, length).unwrap());
        }
    }
    values.push(incant_localization::format_number(locale, -1234.5).unwrap());
    values
}
#[test]
fn real_rust_guest_formats_all_lengths_boundaries_and_next_committed_locale_without_pseudo() {
    let mut native = StaticModule::new(embedded::linked("rust-formatting").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&bytes("rust-formatting")).unwrap();
    let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
    for (tick, locale, next) in [
        (1, "en", "fr"),
        (2, "fr", "ar"),
        (3, "ar", "de-CH"),
        (4, "de-CH", "en"),
    ] {
        let expected = expected(locale);
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        assert_eq!(a.localization().settings().locale, next);
        assert!(a.localization().settings().pseudo);
        let logs = native.logs().unwrap().entries();
        assert_eq!(logs, dev.logs().unwrap().entries());
        assert_eq!(
            logs[(tick - 1) * 15..]
                .iter()
                .map(|r| r.message.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert!(
            logs[(tick - 1) * 15..]
                .iter()
                .all(|r| r.tick == tick as u64)
        );
        assert_eq!(
            a.inspect(StableId([1; 16])).unwrap().transform.translation[0],
            tick as f64
        );
        assert!(a.inspect(StableId(77u128.to_be_bytes())).is_some());
        if tick == 2 {
            let old = dev.logs().unwrap().entries().to_vec();
            assert!(dev.reload(&bytes("formatting-start")).is_err());
            assert_eq!(dev.logs().unwrap().entries(), old);
            dev.reload(&bytes("rust-formatting")).unwrap();
            assert_eq!(dev.logs().unwrap().entries(), old);
        }
    }
}
#[test]
fn raw_unaligned_copies_use_actual_locale_and_do_not_pseudo_wrap_primitive_text() {
    for locale in ["en", "fr", "ar", "de-CH"] {
        let mut settings = settings(locale);
        settings.pseudo = true;
        let scene = scene()
            .with_localization(NativeLocalization::new(tables(), settings).unwrap())
            .unwrap();
        let mut native = StaticModule::new(embedded::linked("formatting").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes("formatting")).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene.clone()), NativeWorld::new(scene));
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        assert_eq!(a.localization().settings().locale, "fr");
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        let logs = native.logs().unwrap().entries();
        assert_eq!(
            logs[0].message,
            incant_localization::format_number(locale, -1234.5).unwrap()
        );
        assert_eq!(
            logs[1].message,
            incant_localization::format_date(
                locale,
                &CalendarDate {
                    year: 2024,
                    month: 2,
                    day: 29
                },
                DateLength::Long
            )
            .unwrap()
        );
    }
}
#[test]
fn raw_invalid_domain_tags_capacity_pointer_quota_epoch_and_traps_roll_back_all_work() {
    for mode in 2..=19 {
        let mut native = StaticModule::new(embedded::linked("formatting").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes("formatting")).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.save_bytes().unwrap();
        assert!(
            native.tick(&mut a, mode as f64 * DT).is_err(),
            "static {mode}"
        );
        assert!(dev.tick(&mut b, mode as f64 * DT).is_err(), "dev {mode}");
        assert_eq!(a.save_bytes().unwrap(), before, "static {mode}");
        assert_eq!(b.save_bytes().unwrap(), before, "dev {mode}");
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
    }
    assert!(StaticModule::new(embedded::linked("formatting-start").unwrap()).is_err());
    assert!(DevModuleHost::new(&bytes("formatting-start")).is_err());
}
#[test]
fn ignored_rust_sdk_errors_and_callback_failure_are_terminal_and_atomic() {
    for kind in [
        "capacity", "number", "range", "date", "year", "quota", "callback",
    ] {
        let name = format!("rust-formatting-{kind}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes(&name)).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.save_bytes().unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "static {kind}");
        assert!(dev.tick(&mut b, DT).is_err(), "dev {kind}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(native.logs().is_err() && dev.logs().is_err());
        assert!(native.tick(&mut a, DT).is_err() && dev.tick(&mut b, DT).is_err());
    }
}
#[test]
fn static_native_session_observer_receives_actual_guest_formatted_text() {
    let mut session = incant_script::NativeSession::new(
        scene(),
        embedded::linked("rust-formatting").unwrap(),
        Default::default(),
    )
    .unwrap();
    for locale in ["en", "fr", "ar", "de-CH"] {
        session.tick(&[]).unwrap();
        let logs = session.logs().unwrap().entries();
        let actual = logs[logs.len() - 15..]
            .iter()
            .map(|r| r.message.clone())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected(locale));
    }
    // Re-entering en tries to create an existing entity: previously committed
    // logs remain hidden after native acceptance failure.
    assert!(session.tick(&[]).is_err());
    assert!(session.logs().is_err());
}
#[test]
fn unknown_format_namespace_and_wrong_signature_refuse_before_execution() {
    for import in [
        r#"(import "incant_localization_v2" "format" (func (param i64 i32 i32) (result i32)))"#,
        r#"(import "incant_localization_v1" "format" (func (param i64 f64) (result i32)))"#,
    ] {
        let bytes = wat::parse_str(format!("(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) (i32.const 1)) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}
