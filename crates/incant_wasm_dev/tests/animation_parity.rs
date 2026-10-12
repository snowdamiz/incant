#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/animations.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 60.;
fn bytes(name: &str) -> Vec<u8> {
    if name.starts_with("rust-animation") {
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
fn states(world: &mut NativeWorld) -> Vec<AnimationSnapshot> {
    let mut rows = Vec::new();
    world.snapshot_animations(&mut rows).unwrap();
    rows.sort_by_key(|r| r.id);
    rows
}
#[test]
fn actual_rust_controls_and_bulk_readback_match_development_and_static_native_hosts() {
    let mut native = StaticModule::new(embedded::linked("rust-animation").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&bytes("rust-animation")).unwrap();
    let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
    // Independent expected samples after each world step + atomic callback.
    let expected = [
        (Some(0), 0.75, false, false, 1.0),
        (Some(0), 0.75, true, false, -2.),
        (Some(0), 0.01, true, false, -2.),
        (Some(1), 0., true, false, -2.),
        (None, 0., true, false, -2.),
        (Some(0), 0.99, true, false, 2.),
        (Some(0), 1., false, true, 2.),
        (Some(0), 0.5, true, false, 2.),
        (Some(0), 0.5 + 2. * DT, true, false, 2.),
    ];
    for (index, (clip, seconds, playing, completed, speed)) in expected.into_iter().enumerate() {
        a.step().unwrap();
        b.step().unwrap();
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(states(&mut a), states(&mut b));
        assert_eq!(a.snapshot(), b.snapshot());
        let state = a.animation_state(id(1)).unwrap();
        assert_eq!(state.sample.clip, clip);
        assert!(
            (state.sample.seconds - seconds).abs() < 1e-12,
            "turn {index}: {state:?}"
        );
        assert_eq!(state.playing, playing);
        assert_eq!(state.completed, completed);
        assert_eq!(state.speed, if index == 0 { 1. } else { speed });
        assert_eq!(state.looping, (2..=4).contains(&index));
        assert_eq!(
            a.inspect(id(1)).unwrap().transform.translation[0],
            (index + 1) as f64
        );
        let independent = a.animation_state(id(2)).unwrap();
        assert!((independent.sample.seconds - (index + 1) as f64 * DT).abs() < 1e-12);
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        assert_eq!(native.logs().unwrap().entries().len(), index + 1);
    }
}
#[test]
fn ignored_sdk_errors_and_native_commit_refusals_roll_back_all_staged_work() {
    for suffix in ["capacity", "number", "rest", "clip", "entity", "callback"] {
        let name = format!("rust-animation-{suffix}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes(&name)).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.snapshot();
        let playback = states(&mut a);
        assert!(native.tick(&mut a, DT).is_err(), "static {suffix}");
        assert!(dev.tick(&mut b, DT).is_err(), "dev {suffix}");
        assert_eq!(a.snapshot(), before);
        assert_eq!(b.snapshot(), before);
        assert_eq!(states(&mut a), playback);
        assert_eq!(states(&mut b), playback);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
        assert!(native.tick(&mut a, DT).is_err());
        assert!(dev.tick(&mut b, DT).is_err());
    }
}
#[test]
fn raw_unaligned_frames_work_and_pointer_tag_stale_traps_reject_prefixes() {
    for mode in 0..=7 {
        let mut native = StaticModule::new(embedded::linked("animations").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes("animations")).unwrap();
        let (mut a, mut b) = (NativeWorld::new(scene()), NativeWorld::new(scene()));
        let before = a.snapshot();
        let playback = states(&mut a);
        let nr = native.tick(&mut a, (mode + 1) as f64 * DT);
        let dr = dev.tick(&mut b, (mode + 1) as f64 * DT);
        if mode == 0 {
            nr.unwrap();
            dr.unwrap();
            assert_eq!(a.snapshot(), b.snapshot());
            assert_eq!(states(&mut a), states(&mut b));
            assert!(!a.animation_state(id(1)).unwrap().playing);
            assert_eq!(a.inspect(id(1)).unwrap().transform.translation[0], 77.);
        } else {
            assert!(nr.is_err(), "static mode {mode}");
            assert!(dr.is_err(), "dev mode {mode}");
            assert_eq!(a.snapshot(), before);
            assert_eq!(b.snapshot(), before);
            assert_eq!(states(&mut a), playback);
            assert_eq!(states(&mut b), playback);
            assert!(native.is_failed() && dev.is_failed());
        }
    }
}
#[test]
fn active_tick_version_and_signature_contracts_are_enforced_before_execution() {
    assert!(StaticModule::new(embedded::linked("animations-start").unwrap()).is_err());
    assert!(DevModuleHost::new(&bytes("animations-start")).is_err());
    for import in [
        r#"(import "incant_animation_v3" "snapshot_len" (func (param i64)(result i32)))"#,
        r#"(import "incant_animation_v1" "control" (func (param i64 i32 i32)(result i32)))"#,
        r#"(import "incant_animation_v1" "read_snapshot" (func (param i64 i32)(result i32)))"#,
    ] {
        let bytes=wat::parse_str(format!("(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) i32.const 1) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}
