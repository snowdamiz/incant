#![cfg(feature = "static-conformance")]
#[path = "support/audio_pcm.rs"]
mod audio;
#[path = "../../incant_wasm_runtime/tests/support/audio_transport.rs"]
mod fixture;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
#[test]
fn real_rust_hosts_match_all_declaration_fields_native_transactions_and_mixed_pcm() {
    let name = "rust-audio-transport";
    let mut compiled = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    let mut a = NativeWorld::new(fixture::scene());
    let mut b = NativeWorld::new(fixture::scene());
    let mut native = NativeWorld::new(fixture::scene());
    let cache = tempfile::tempdir().unwrap();
    let keys = [
        audio::clip(cache.path(), 22050, "static"),
        audio::clip(cache.path(), 32000, "stream"),
    ];
    let mut a_audio = audio::host(&a, cache.path(), &keys);
    let mut b_audio = audio::host(&b, cache.path(), &keys);
    let mut native_audio = audio::host(&native, cache.path(), &keys);
    let (mut nonzero, mut silent) = (0, 0);
    for turn in 0..96 {
        a.step().unwrap();
        b.step().unwrap();
        native.step().unwrap();
        let expected = fixture::observations(&native);
        fixture::oracle(&mut native, turn);
        compiled
            .tick(&mut a, 1. / 60.)
            .unwrap_or_else(|e| panic!("compiled turn{turn}: {e}"));
        dev.tick(&mut b, 1. / 60.)
            .unwrap_or_else(|e| panic!("development turn{turn}: {e}"));
        assert_eq!(
            a.save_bytes().unwrap(),
            native.save_bytes().unwrap(),
            "compiled turn{turn}"
        );
        assert_eq!(
            b.save_bytes().unwrap(),
            native.save_bytes().unwrap(),
            "development turn{turn}"
        );
        let logs = compiled.logs().unwrap().entries();
        assert_eq!(logs, dev.logs().unwrap().entries());
        for (part, record) in logs[logs.len() - 6..].iter().enumerate() {
            let value: serde_json::Value = serde_json::from_str(&record.message).unwrap();
            assert_eq!(value["tick"], turn + 1);
            assert_eq!(value["part"], part);
            let actual = value["values"].as_array().unwrap();
            assert_eq!(actual.len(), 31);
            for (actual, expected) in actual.iter().zip(&expected[part * 31..part * 31 + 31]) {
                assert_eq!(actual.as_f64().unwrap(), *expected);
            }
        }
        let reference = audio::render(&mut native_audio, &native);
        assert_eq!(
            audio::render(&mut a_audio, &a),
            reference,
            "compiled PCM turn{turn}"
        );
        assert_eq!(
            audio::render(&mut b_audio, &b),
            reference,
            "development PCM turn{turn}"
        );
        assert!(reference.iter().all(|s| s.is_finite()));
        if reference.iter().any(|s| s.abs() > 0.001) {
            nonzero += 1;
        }
        if reference.iter().all(|s| *s == 0.) {
            silent += 1;
        }
        if turn == 47 {
            dev.reload(&guest(name)).unwrap();
        }
    }
    assert!(
        nonzero > 40 && silent >= 1,
        "nonzero{nonzero} silent{silent}"
    );
}
#[test]
fn actual_caught_sdk_errors_traps_and_late_native_refusals_discard_audio_numeric_and_logs() {
    for mode in [
        "scratch", "snapshot", "seek", "gain", "owner", "role", "quota", "late", "callback", "trap",
    ] {
        let name = format!("rust-audio-{mode}");
        let mut compiled = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let mut a = NativeWorld::new(fixture::scene());
        let mut b = NativeWorld::new(fixture::scene());
        a.step().unwrap();
        b.step().unwrap();
        let before = a.save_bytes().unwrap();
        assert!(compiled.tick(&mut a, 1. / 60.).is_err(), "compiled{mode}");
        assert!(dev.tick(&mut b, 1. / 60.).is_err(), "development{mode}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(compiled.is_failed() && dev.is_failed());
        assert!(compiled.logs().is_err() && dev.logs().is_err());
    }
    let name = "rust-audio-prefix";
    let mut compiled = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    let mut a = NativeWorld::new(fixture::scene());
    let mut b = NativeWorld::new(fixture::scene());
    a.step().unwrap();
    b.step().unwrap();
    let before = a.save_bytes().unwrap();
    compiled.tick(&mut a, 1. / 60.).unwrap();
    dev.tick(&mut b, 1. / 60.).unwrap();
    assert_ne!(a.save_bytes().unwrap(), before);
    assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
    assert_eq!(compiled.logs().unwrap().entries().len(), 1);
    assert_eq!(
        compiled.logs().unwrap().entries(),
        dev.logs().unwrap().entries()
    );
}

#[test]
fn raw_audio_bounds_canonical_batches_and_shared_quotas_refuse_with_whole_prefix_rollback() {
    let bytes = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/audio-transport.wat"
    ))
    .unwrap();
    for mode in 0..29 {
        let mut compiled = StaticModule::new(embedded::linked("audio-transport").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes).unwrap();
        let mut a = NativeWorld::new(fixture::scene());
        let mut b = NativeWorld::new(fixture::scene());
        let before = a.save_bytes().unwrap();
        let dt = f64::from(mode + 1) / 64.;
        let x = compiled.tick(&mut a, dt);
        let y = dev.tick(&mut b, dt);
        if matches!(mode, 0 | 28) {
            x.unwrap();
            y.unwrap();
            let source = a.audio_nodes().nth(1).unwrap().0.source.as_ref().unwrap();
            assert_eq!(
                source.transport_epoch,
                (1_u64 << 53) + 1 + if mode == 28 { 1024 } else { 1 }
            );
            assert_eq!(source.start_seconds, 0.);
            assert!(source.playing);
            // Numeric chunk zero follows native archetype order, not entity order.
            assert_eq!(
                a.snapshot()
                    .iter()
                    .filter(|r| r.transform.translation[0] == 99.)
                    .count(),
                1
            );
            assert_ne!(a.save_bytes().unwrap(), before);
            assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        } else {
            let x = x.unwrap_err().to_string();
            let y = y.unwrap_err().to_string();
            let expected = match mode {
                1 => Some("inactive or expired ECS epoch"),
                7 => Some("host call quota"),
                8 => Some("transfer quota"),
                9 => Some("command quota"),
                10 | 22 | 23 => Some("audio command length"),
                11..=20 | 26 | 27 => Some("audio command values"),
                _ => None,
            };
            if let Some(expected) = expected {
                assert!(x.contains(expected), "compiled {mode}: {x}");
                assert!(y.contains(expected), "development {mode}: {y}");
            }
            assert_eq!(a.save_bytes().unwrap(), before, "compiled {mode}");
            assert_eq!(b.save_bytes().unwrap(), before, "development {mode}");
        }
    }
}

#[test]
fn unknown_audio_versions_signatures_and_names_refuse_before_instantiation() {
    for import in [
        r#"(import "incant_audio_v2" "snapshot_len" (func(param i64)(result i32)))"#,
        r#"(import "incant_audio_v1" "read_snapshot" (func(param i64 i32)(result i32)))"#,
        r#"(import "incant_audio_v1" "command" (func(param i64 i32 i32)))"#,
        r#"(import "incant_audio_v1" "cursor" (func(param i64)(result i32)))"#,
    ] {
        let bytes = wat::parse_str(format!(
            "(module {import} (memory(export \"memory\")1) (func(export \"incant_abi\")(result i32)i32.const 1)(func(export \"update\")(param i64 f64)))"
        ))
        .unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}
