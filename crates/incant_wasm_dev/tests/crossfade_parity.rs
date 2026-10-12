#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/animations.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn guest(name: &str) -> Vec<u8> {
    if name.starts_with("rust-crossfade") {
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
fn states(w: &mut NativeWorld) -> Vec<AnimationSnapshot> {
    let mut rows = Vec::new();
    w.snapshot_animations(&mut rows).unwrap();
    rows.sort_by_key(|r| r.id);
    rows
}
#[test]
fn real_rust_guest_observes_both_clocks_pause_captured_source_and_cancellation_on_both_hosts() {
    let mut a = NativeWorld::new(scene());
    let mut b = NativeWorld::new(scene());
    let mut native = StaticModule::new(embedded::linked("rust-crossfade").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest("rust-crossfade")).unwrap();
    for turn in 0..65 {
        a.step().unwrap();
        b.step().unwrap();
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(states(&mut a), states(&mut b));
        assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        let state = a.animation_state(id(1)).unwrap();
        if turn == 0 {
            let t = state.transition.unwrap();
            assert_eq!(
                (
                    t.source.clip,
                    t.source.seconds,
                    t.speed,
                    t.playing,
                    t.looping,
                    t.elapsed,
                    t.duration
                ),
                (Some(0), DT, 1., true, false, 0., 0.5)
            );
            assert_eq!(
                (state.sample.seconds, state.speed, state.looping),
                (0.75, -1., true)
            );
        }
        if turn == 1 || turn == 2 {
            let t = state.transition.unwrap();
            assert_eq!(t.elapsed, DT);
            assert_eq!(t.source.seconds, 2. * DT);
            assert_eq!(state.playing, turn == 2);
        }
        if turn == 31 {
            assert!(state.transition.is_none());
        }
        if turn == 35 {
            assert_eq!(state.sample, AnimationSample::REST);
            assert!(state.transition.is_none());
        }
        if turn == 38 {
            let t = state.transition.unwrap();
            assert_eq!(t.source, AnimationSample::REST);
            assert!(!t.playing);
            assert_eq!(t.speed, -1.);
            assert_eq!(state.speed, 2.);
            assert_eq!(t.elapsed, DT);
        }
        if turn == 62 {
            assert_eq!(state.sample, AnimationSample::REST);
            assert!(state.transition.is_none());
        }
        assert_eq!(
            a.inspect(id(1)).unwrap().transform.translation[0],
            f64::from(turn + 1)
        );
    }
}
#[test]
fn actual_guest_continues_from_v5_world_save_in_fresh_hosts_without_module_state_claim() {
    let initial = scene();
    let mut baseline = NativeWorld::new(initial.clone());
    let mut running = StaticModule::new(embedded::linked("rust-crossfade").unwrap()).unwrap();
    for _ in 0..4 {
        baseline.step().unwrap();
        running.tick(&mut baseline, DT).unwrap();
    }
    let saved = baseline.save_bytes().unwrap();
    assert_eq!(&saved[8..12], &5u32.to_le_bytes());
    let mut a = NativeWorld::from_save_bytes(&saved).unwrap();
    let mut b = NativeWorld::from_save_bytes(&saved).unwrap();
    let mut restored = NativeWorld::new(initial);
    restored.restore_bytes(&saved).unwrap();
    let mut native = StaticModule::new(embedded::linked("rust-crossfade").unwrap()).unwrap();
    let mut in_place = StaticModule::new(embedded::linked("rust-crossfade").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest("rust-crossfade")).unwrap();
    for _ in 0..61 {
        baseline.step().unwrap();
        a.step().unwrap();
        b.step().unwrap();
        restored.step().unwrap();
        running.tick(&mut baseline, DT).unwrap();
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        in_place.tick(&mut restored, DT).unwrap();
        let expected = baseline.save_bytes().unwrap();
        assert_eq!(a.save_bytes().unwrap(), expected);
        assert_eq!(b.save_bytes().unwrap(), expected);
        assert_eq!(restored.save_bytes().unwrap(), expected);
    }
    // Full static session snapshots remain explicitly unsupported: no module
    // memory/globals are serialized or fabricated by this world continuation.
    let session = incant_script::NativeSession::new(
        scene(),
        embedded::linked("rust-crossfade").unwrap(),
        Default::default(),
    )
    .unwrap();
    assert!(matches!(
        session.checkpoint_bytes(),
        Err(incant_script::CheckpointError::Unsupported(
            "static module state ABI"
        ))
    ));
}
#[test]
fn real_rust_ignored_sdk_failures_native_retarget_quota_callback_and_trap_rollback() {
    for suffix in [
        "capacity", "number", "duration", "rest", "retarget", "clip", "quota", "callback", "trap",
    ] {
        let name = format!("rust-crossfade-{suffix}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let mut a = NativeWorld::new(scene());
        let mut b = NativeWorld::new(scene());
        let before = a.save_bytes().unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "static {suffix}");
        assert!(dev.tick(&mut b, DT).is_err(), "dev {suffix}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
    }
}
#[test]
fn raw_versioned_unaligned_frames_and_late_errors_match_both_hosts() {
    for mode in 0..=8 {
        let mut native = StaticModule::new(embedded::linked("crossfades").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest("crossfades")).unwrap();
        let mut a = NativeWorld::new(scene());
        let mut b = NativeWorld::new(scene());
        let before = a.save_bytes().unwrap();
        let nr = native.tick(&mut a, (mode + 1) as f64 * DT);
        let dr = dev.tick(&mut b, (mode + 1) as f64 * DT);
        if mode == 0 {
            nr.unwrap();
            dr.unwrap();
            assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
            let t = a.animation_state(id(1)).unwrap().transition.unwrap();
            assert_eq!(t.duration, 0.5);
            assert_eq!(t.elapsed, 0.);
            assert_eq!(a.inspect(id(1)).unwrap().transform.translation[0], 77.);
        } else {
            assert!(nr.is_err(), "static mode {mode}");
            assert!(dr.is_err(), "dev mode {mode}");
            assert_eq!(a.save_bytes().unwrap(), before);
            assert_eq!(b.save_bytes().unwrap(), before);
            assert!(native.is_failed() && dev.is_failed());
        }
    }
}
#[test]
fn additive_import_versions_and_signatures_refuse_unsupported_contracts() {
    for import in [
        r#"(import "incant_animation_v6" "snapshot_len" (func (param i64)(result i32)))"#,
        r#"(import "incant_animation_v2" "read_snapshot" (func (param i64 i32)(result i32)))"#,
        r#"(import "incant_animation_v2" "control" (func (param i64 i32 i32)(result i32)))"#,
    ] {
        let bytes=wat::parse_str(format!("(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) i32.const 1) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}
