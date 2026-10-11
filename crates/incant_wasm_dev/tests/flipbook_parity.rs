#![cfg(feature = "static-conformance")]
#[path = "../../incant_runtime/tests/support/flipbooks.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1. / 64.;
fn source(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../incant_wasm_runtime/fixtures")
            .join(format!("{name}.wat")),
    )
    .unwrap()
}
fn states(world: &mut NativeWorld) -> Vec<FlipbookSnapshot> {
    let mut rows = Vec::new();
    world.snapshot_flipbooks(&mut rows).unwrap();
    rows.sort_by_key(|r| r.id);
    rows
}
#[test]
fn real_development_and_static_guests_share_bulk_readback_and_ordered_controls() {
    let mut a = NativeWorld::new(scene(2));
    let mut b = NativeWorld::new(scene(2));
    let mut native = StaticModule::new(embedded::linked("flipbooks").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&wat::parse_str(source("flipbooks")).unwrap()).unwrap();
    for (steps, frame, playing) in [
        (1, 0, false),
        (1, 2, false),
        (1, 2, true),
        (1, 0, true),
        (4, 1, true),
        (8, 0, true),
    ] {
        for _ in 0..steps {
            a.step().unwrap();
            b.step().unwrap();
        }
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(states(&mut a), states(&mut b));
        assert_eq!(a.snapshot(), b.snapshot());
        let state = a.flipbook_state(id(1)).unwrap();
        assert_eq!(state.frame(), frame);
        assert_eq!(state.playing(), playing);
    }
    assert!(a.flipbook_state(id(2)).unwrap().completed());
}
#[test]
fn real_host_failures_revoke_and_reject_numeric_and_flipbook_prefixes() {
    for name in ["invalid", "tag", "stale", "trap", "short", "oob"] {
        let name = format!("flipbooks-{name}");
        let mut a = NativeWorld::new(scene(2));
        let mut b = NativeWorld::new(scene(2));
        let before = a.snapshot();
        let playback = states(&mut a);
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&wat::parse_str(source(&name)).unwrap()).unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "{name}");
        assert!(dev.tick(&mut b, DT).is_err(), "{name}");
        assert_eq!(a.snapshot(), before);
        assert_eq!(b.snapshot(), before);
        assert_eq!(states(&mut a), playback);
        assert_eq!(states(&mut b), playback);
        assert!(native.is_failed());
        assert!(dev.is_failed());
        assert!(native.tick(&mut a, DT).is_err());
        assert!(dev.tick(&mut b, DT).is_err());
    }
}
#[test]
fn new_namespace_requires_an_active_tick_and_unsupported_versions_fail() {
    assert!(StaticModule::new(embedded::linked("flipbooks-start").unwrap()).is_err());
    assert!(DevModuleHost::new(&wat::parse_str(source("flipbooks-start")).unwrap()).is_err());
    assert!(
        DevModuleHost::new(
            &wat::parse_str(
                source("flipbooks").replace("incant_flipbook_v1", "incant_flipbook_v2")
            )
            .unwrap()
        )
        .is_err()
    );
}
#[test]
fn actual_rust_guest_consumes_input_to_control_bulk_playback_in_both_hosts() {
    use incant_input::{
        InputAction, InputBinding, InputEvent, InputRuntime, KeyCode, ValidatedActions,
    };
    let actions = ValidatedActions::new(
        [
            (
                "flip_pause".into(),
                InputAction::Button {
                    bindings: vec![InputBinding::Key {
                        code: KeyCode::KeyP,
                        scale: [1., 0.],
                    }],
                    threshold: 0.5,
                },
            ),
            (
                "flip_play".into(),
                InputAction::Button {
                    bindings: vec![InputBinding::Key {
                        code: KeyCode::KeyL,
                        scale: [1., 0.],
                    }],
                    threshold: 0.5,
                },
            ),
        ]
        .into(),
    )
    .unwrap();
    let mut input = InputRuntime::default();
    let mut a = NativeWorld::new(scene(2));
    let mut b = NativeWorld::new(scene(2));
    let mut native = StaticModule::new(embedded::linked("flipbook-game").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(include_bytes!(env!("INCANT_FLIPBOOK_GUEST"))).unwrap();
    for tick in 1..=20 {
        let events = match tick {
            1 => vec![InputEvent::Key {
                code: KeyCode::KeyP,
                down: true,
            }],
            10 => vec![InputEvent::Key {
                code: KeyCode::KeyL,
                down: true,
            }],
            _ => vec![],
        };
        input.advance_validated(&events, DT, &actions).unwrap();
        a.step().unwrap();
        b.step().unwrap();
        native.tick_with_input(&mut a, DT, &input).unwrap();
        dev.tick_with_input(&mut b, DT, &input).unwrap();
        assert_eq!(states(&mut a), states(&mut b));
        assert_eq!(a.snapshot(), b.snapshot());
        let expected = if tick < 10 { 0 } else { ((tick - 9) / 4) % 3 };
        assert_eq!(a.flipbook_state(id(1)).unwrap().frame(), expected);
        assert_eq!(a.flipbook_state(id(1)).unwrap().playing(), tick >= 10);
    }
}
