#![cfg(feature = "static-conformance")]
#[path = "../../incant_runtime/tests/support/localization.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
#[test]
fn real_guest_pages_match_native_coverage_across_commits_save_load_and_reload() {
    let mut native =
        StaticModule::new(embedded::linked("rust-localization-report").unwrap()).unwrap();
    let guest = bytes("rust-localization-report");
    let mut dev = DevModuleHost::new(&guest).unwrap();
    let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
    let mut expected = Vec::new();
    for (tick, locale, next) in [
        (1, "en", "fr"),
        (2, "fr", "ar"),
        (3, "ar", "zh-Hant"),
        (4, "zh-Hant", "en"),
    ] {
        let coverage = a.localization().report(Default::default()).unwrap();
        expected.extend([locale.to_owned(), coverage.total.to_string()]);
        for row in coverage.entries {
            expected.extend([
                row.key,
                row.requested_locale,
                row.resolved_locale.unwrap_or("none".into()),
            ]);
        }
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        assert_eq!(a.localization().settings().locale, next);
        assert!(a.localization().settings().pseudo);
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        assert_eq!(
            native
                .logs()
                .unwrap()
                .entries()
                .iter()
                .map(|r| r.message.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            a.inspect(StableId([1; 16])).unwrap().transform.translation[0],
            tick as f64
        );
        if tick == 2 {
            a.restore_bytes(&a.save_bytes().unwrap()).unwrap();
            b.restore_bytes(&b.save_bytes().unwrap()).unwrap();
            dev.reload(&guest).unwrap();
        }
    }
}
#[test]
fn ignored_sdk_report_errors_and_callback_failure_roll_back_the_complete_batch() {
    for kind in ["offset", "limit", "capacity", "quota", "callback"] {
        let name = format!("rust-localization-report-{kind}");
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
fn report_import_rejects_unknown_namespaces_and_wrong_signatures_before_execution() {
    for import in [
        r#"(import "incant_localization_v2" "report" (func (param i64 i32 i32) (result i32)))"#,
        r#"(import "incant_localization_v1" "report" (func (param i64 f64) (result i32)))"#,
    ] {
        let bytes = wat::parse_str(format!("(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) (i32.const 1)) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}
