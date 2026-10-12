#![cfg(feature = "static-conformance")]
#[path = "../../incant_runtime/tests/support/localization.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn scene_with_date() -> CookedScene {
    let mut tables = tables();
    tables.get_mut(TABLE).unwrap().messages.insert(
        "dated".into(),
        [
            ("en".into(), "Date {date, date, long}".into()),
            ("fr".into(), "Date {date, date, long}".into()),
        ]
        .into(),
    );
    scene()
        .with_localization(NativeLocalization::new(tables, settings("en")).unwrap())
        .unwrap()
}
fn bytes(name: &str) -> Vec<u8> {
    if name.starts_with("rust-localization") {
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
#[test]
fn actual_rust_guest_shares_native_icu_plural_date_unicode_fallback_pseudo_and_missing_semantics() {
    let mut native = StaticModule::new(embedded::linked("rust-localization").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&bytes("rust-localization")).unwrap();
    let (mut a, mut b) = (
        NativeWorld::new(scene_with_date()),
        NativeWorld::new(scene_with_date()),
    );
    for (tick, locale, next) in [
        (1, "en", "fr"),
        (2, "fr", "ru"),
        (3, "ru", "de-CH"),
        (4, "de-CH", "en"),
    ] {
        let catalog = a.localization().clone();
        let expected = ["hello", "items", "party", "rank", "dated", "absent"].map(|key| {
            let mut request = request(key);
            request.arguments = [
                (
                    "name".into(),
                    incant_localization::MessageArgument::Text("雪🙂".into()),
                ),
                (
                    "n".into(),
                    incant_localization::MessageArgument::Number(21.),
                ),
                (
                    "gender".into(),
                    incant_localization::MessageArgument::Text("female".into()),
                ),
                (
                    "date".into(),
                    incant_localization::MessageArgument::Date(CalendarDate {
                        year: 2024,
                        month: 2,
                        day: 29,
                    }),
                ),
            ]
            .into();
            catalog.localize(&request).unwrap().text
        });
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(a.localization().settings(), b.localization().settings());
        assert_eq!(a.localization().settings().locale, next);
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        let rows = &native.logs().unwrap().entries()[(tick - 1) * 7..];
        assert_eq!(rows[0].message, locale);
        if tick == 1 {
            assert_eq!(rows[1].message, "Hello 雪🙂");
            assert_eq!(rows[2].message, "21 items");
            assert_eq!(rows[3].message, "She and 20 guests");
            assert_eq!(rows[4].message, "21st");
        }
        if tick == 4 {
            assert!(rows[1].message.contains("雪🙂"));
            assert!(rows[1].message.starts_with("[!! "));
        }
        assert!(rows.iter().all(|r| r.tick == tick as u64));
        assert_eq!(
            rows[1..]
                .iter()
                .map(|r| r.message.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            a.inspect(StableId([1; 16])).unwrap().transform.translation[0],
            tick as f64
        );
        assert!(a.inspect(StableId(77u128.to_be_bytes())).is_some());
    }
}
#[test]
fn real_wasmtime_and_static_traps_reject_mixed_numeric_structural_locale_and_log_prefixes() {
    for mode in 2..=12 {
        let mut native = StaticModule::new(embedded::linked("localization").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes("localization")).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.snapshot();
        assert!(
            native.tick(&mut a, mode as f64 * DT).is_err(),
            "static {mode}"
        );
        assert!(dev.tick(&mut b, mode as f64 * DT).is_err(), "dev {mode}");
        assert_eq!(a.snapshot(), before);
        assert_eq!(b.snapshot(), before);
        assert_eq!(a.localization().settings().locale, "en");
        assert_eq!(b.localization().settings().locale, "en");
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
    }
}
#[test]
fn ignored_sdk_errors_and_callback_failure_cannot_publish_any_staged_work() {
    for kind in ["capacity", "number", "duplicate", "callback"] {
        let name = format!("rust-localization-{kind}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes(&name)).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.snapshot();
        assert!(native.tick(&mut a, DT).is_err(), "static {kind}");
        assert!(dev.tick(&mut b, DT).is_err(), "dev {kind}");
        assert_eq!(a.snapshot(), before);
        assert_eq!(b.snapshot(), before);
        assert_eq!(a.localization().settings().locale, "en");
        assert_eq!(b.localization().settings().locale, "en");
        assert!(native.logs().is_err() && dev.logs().is_err());
    }
}
#[test]
fn actual_unaligned_copied_frames_and_reload_use_the_current_committed_locale() {
    let mut dev = DevModuleHost::new(&bytes("localization")).unwrap();
    let mut native = StaticModule::new(embedded::linked("localization").unwrap()).unwrap();
    let (mut a, mut b) = (
        NativeWorld::new(scene_with_date()),
        NativeWorld::new(scene_with_date()),
    );
    native.tick(&mut a, DT).unwrap();
    dev.tick(&mut b, DT).unwrap();
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.localization().settings().locale, "fr");
    assert!(StaticModule::new(embedded::linked("localization-start").unwrap()).is_err());
    assert!(dev.reload(&bytes("localization-start")).is_err());
    assert!(!dev.is_failed());
    assert_eq!(dev.logs().unwrap().entries().len(), 1);
    // A failed update keeps the committed locale/history; replacement has no
    // private result buffer and takes its snapshot from this world's locale.
    assert!(dev.tick(&mut b, 3. * DT).is_err());
    assert_eq!(b.localization().settings().locale, "fr");
    dev.reload(&bytes("rust-localization")).unwrap();
    assert_eq!(dev.logs().unwrap().entries().len(), 1);
    dev.tick(&mut b, DT).unwrap();
    let rows = &dev.logs().unwrap().entries()[1..];
    assert_eq!(rows[0].message, "fr");
    assert!(rows.iter().all(|r| r.tick == 2));
    assert_eq!(b.localization().settings().locale, "ru");
}
#[test]
fn static_native_session_exposes_actual_localized_guest_logs_through_public_observer() {
    let mut session = incant_script::NativeSession::new(
        scene_with_date(),
        embedded::linked("rust-localization").unwrap(),
        Default::default(),
    )
    .unwrap();
    session.tick(&[]).unwrap();
    let logs = serde_json::to_value(session.logs().unwrap()).unwrap();
    assert_eq!(logs["entries"][0]["message"], "en");
    assert_eq!(logs["entries"][1]["message"], "Hello 雪🙂");
    assert_eq!(logs["entries"].as_array().unwrap().len(), 7);
    assert_eq!(logs["dropped"], 0);
    assert_eq!(
        session.world().unwrap().localization().settings().locale,
        "fr"
    );
    for _ in 0..3 {
        session.tick(&[]).unwrap();
    }
    // The next en callback queues an already-live entity and fails native
    // acceptance. Prior localized logs remain hidden after terminal failure.
    assert!(session.tick(&[]).is_err());
    assert!(session.logs().is_err());
    assert!(session.world().is_err());
}
#[test]
fn versions_and_signature_mismatches_are_refused_before_execution() {
    for import in [
        r#"(import "incant_localization_v2" "locale" (func (param i64) (result i32)))"#,
        r#"(import "incant_localization_v1" "localize" (func (param i64 i32) (result i32)))"#,
        r#"(import "incant_localization_v1" "set_locale" (func (param i64 i32 i32) (result i32)))"#,
    ] {
        let bytes = wat::parse_str(format!("(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) (i32.const 1)) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}
