#![cfg(feature = "static-conformance")]
#[path = "../../incant_runtime/tests/support/flipbooks.rs"]
mod fixture;
use fixture::*;
use incant_input::{
    InputAction, InputBinding, InputEvent, InputRuntime, KeyCode, ValidatedActions,
};
use incant_runtime::*;
use incant_types::Velocity;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 64.;
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
fn states(world: &mut NativeWorld) -> Vec<FlipbookSnapshot> {
    let mut rows = Vec::new();
    world.snapshot_flipbooks(&mut rows).unwrap();
    rows.sort_by_key(|r| r.id);
    rows
}
#[test]
fn public_rust_sample_runs_numeric_input_structural_and_bulk_playback_on_both_hosts() {
    let mut a = NativeWorld::new(scene(2));
    let mut b = NativeWorld::new(scene(2));
    for w in [&mut a, &mut b] {
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::SetVelocity {
                entity: id(1),
                velocity: Some(Velocity { linear: [0.; 3] }),
            })
            .unwrap();
        w.apply(&mut commands).unwrap();
    }
    let mut native = StaticModule::new(embedded::linked("rust-sdk").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest("rust-sdk")).unwrap();
    let actions = ValidatedActions::new(
        [(
            "toggle".into(),
            InputAction::Button {
                bindings: vec![InputBinding::Key {
                    code: KeyCode::Space,
                    scale: [1., 0.],
                }],
                threshold: 0.5,
            },
        )]
        .into(),
    )
    .unwrap();
    let mut input = InputRuntime::default();
    for tick in 1..=12 {
        let events = match tick {
            1 | 3 => vec![InputEvent::Key {
                code: KeyCode::Space,
                down: true,
            }],
            2 | 4 => vec![InputEvent::Key {
                code: KeyCode::Space,
                down: false,
            }],
            _ => vec![],
        };
        input.advance_validated(&events, DT, &actions).unwrap();
        a.step().unwrap();
        b.step().unwrap();
        native.tick_with_input(&mut a, DT, &input).unwrap();
        dev.tick_with_input(&mut b, DT, &input).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        let logs = native.logs().unwrap();
        assert_eq!(logs.entries().len(), tick as usize);
        let log = logs.entries().last().unwrap();
        assert_eq!(log.tick, u64::from(tick));
        assert_eq!(log.level, incant_wasm_runtime::LogLevel::Info);
        assert_eq!(log.message, "Ready 雪🙂\n\t");
        assert_eq!(states(&mut a), states(&mut b));
        let pose = a.inspect(id(1)).unwrap();
        assert_eq!(
            pose.transform.translation,
            [tick as f64 * DT, (tick - 1) as f64 * DT * 0.5, 0.]
        );
        assert_eq!(pose.velocity.unwrap().linear, [0., 0.5, 0.]);
        if tick == 1 {
            let child = a.inspect(id(99)).unwrap();
            assert_eq!(child.parent, Some(id(1)));
            assert_eq!(child.velocity.unwrap().linear, [1., 0., 0.]);
        } else {
            assert!(a.inspect(id(99)).is_none());
        }
        let state = a.flipbook_state(id(1)).unwrap();
        assert_eq!(state.playing(), tick >= 3);
        assert_eq!(
            state.frame(),
            if tick < 3 { 2 } else { ((tick - 3) / 4) % 3 }
        );
    }
}
#[test]
fn callback_error_ignored_capacity_malformed_snapshot_and_old_epoch_cannot_leak_prefixes() {
    for suffix in [
        "callback-error",
        "capacity",
        "malformed",
        "expired",
        "log-bounds",
        "log-invalid",
    ] {
        let name = format!("rust-sdk-{suffix}");
        let mut a = NativeWorld::new(scene(2));
        let mut b = NativeWorld::new(scene(2));
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        if suffix == "expired" {
            native.tick(&mut a, DT).unwrap();
            dev.tick(&mut b, DT).unwrap();
            assert_eq!(a.inspect(id(1)).unwrap().transform.translation[0], 42.);
        }
        let before = a.snapshot();
        let playback = states(&mut a);
        assert!(native.tick(&mut a, DT).is_err(), "{suffix}");
        assert!(dev.tick(&mut b, DT).is_err(), "{suffix}");
        assert_eq!(a.snapshot(), before, "static {suffix}");
        assert_eq!(b.snapshot(), before, "dev {suffix}");
        assert_eq!(states(&mut a), playback);
        assert_eq!(states(&mut b), playback);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
        // Successful reload reveals only the previously committed prefix.
        dev.reload(&guest("rust-sdk")).unwrap();
        assert_eq!(
            dev.logs().unwrap().entries().len(),
            usize::from(suffix == "expired")
        );
        if suffix == "expired" {
            assert_eq!(dev.logs().unwrap().entries()[0].tick, 1);
            assert_eq!(dev.logs().unwrap().entries()[0].message, "staged");
        }
        assert!(native.tick(&mut a, DT).is_err());
    }
}

#[test]
fn raw_unaligned_copies_are_safe_and_overflowing_ranges_are_rejected_in_both_hosts() {
    for name in ["sdk-unaligned", "sdk-overflow"] {
        let source = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../incant_wasm_runtime/fixtures")
                .join(format!("{name}.wat")),
        )
        .unwrap();
        let mut a = NativeWorld::new(scene(2));
        let mut b = NativeWorld::new(scene(2));
        let before = a.snapshot();
        let mut native = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&wat::parse_str(source).unwrap()).unwrap();
        if name == "sdk-unaligned" {
            native.tick(&mut a, DT).unwrap();
            dev.tick(&mut b, DT).unwrap();
            assert_eq!(a.snapshot(), b.snapshot());
            assert_eq!(a.inspect(id(1)).unwrap().transform.translation[0], 7.25);
        } else {
            assert!(native.tick(&mut a, DT).is_err());
            assert!(dev.tick(&mut b, DT).is_err());
            assert_eq!(a.snapshot(), before);
            assert_eq!(b.snapshot(), before);
        }
    }
}
