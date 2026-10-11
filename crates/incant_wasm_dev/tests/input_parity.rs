#![cfg(feature = "static-conformance")]
use incant_input::*;
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::Transform;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{Call, Capability, MAX_INPUT_BYTES, StaticModule, embedded};
const DT: f64 = 1. / 64.;
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([0; 16]),
            64,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }],
        )
        .unwrap(),
    )
}
fn sources() -> [(&'static str, &'static str); 5] {
    [
        (
            "input",
            include_str!("../../incant_wasm_runtime/fixtures/input.wat"),
        ),
        (
            "input-short",
            include_str!("../../incant_wasm_runtime/fixtures/input-short.wat"),
        ),
        (
            "input-oob",
            include_str!("../../incant_wasm_runtime/fixtures/input-oob.wat"),
        ),
        (
            "input-stale",
            include_str!("../../incant_wasm_runtime/fixtures/input-stale.wat"),
        ),
        (
            "input-start",
            include_str!("../../incant_wasm_runtime/fixtures/input-start.wat"),
        ),
    ]
}
#[test]
fn actual_hosts_consume_identical_complete_snapshots_with_edges_and_focus() {
    let mut a = world();
    let mut b = world();
    let mut expected = world();
    let mut native = StaticModule::new(embedded::linked("input").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&wat::parse_str(sources()[0].1).unwrap()).unwrap();
    let mut input = InputRuntime::default();
    let actions = ValidatedActions::new(
        [(
            "jump".into(),
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
    let mut packets = vec![
        vec![
            InputEvent::Key {
                code: KeyCode::Space,
                down: true,
            },
            InputEvent::PointerPosition {
                position: [17., 9.],
            },
            InputEvent::GamepadConnected { id: 9 },
            InputEvent::GamepadButton {
                id: 9,
                button: GamepadButton::South,
                value: 0.75,
            },
            InputEvent::Touch {
                id: 8,
                phase: TouchPhase::Down,
                position: [4., 8.],
            },
        ],
        vec![
            InputEvent::Key {
                code: KeyCode::Space,
                down: false,
            },
            InputEvent::Touch {
                id: 8,
                phase: TouchPhase::Up,
                position: [4., 8.],
            },
            InputEvent::Wheel {
                delta: [1., 2.],
                unit: WheelUnit::Pixels,
            },
        ],
        vec![
            InputEvent::Focus { focused: false },
            InputEvent::GamepadDisconnected { id: 9 },
        ],
        vec![
            InputEvent::Focus { focused: true },
            InputEvent::Key {
                code: KeyCode::F12,
                down: true,
            },
        ],
        vec![],
    ];
    packets.push(
        (0..512)
            .flat_map(|id| {
                [
                    InputEvent::Touch {
                        id,
                        phase: TouchPhase::Down,
                        position: [0., 0.],
                    },
                    InputEvent::Touch {
                        id,
                        phase: TouchPhase::Up,
                        position: [0., 0.],
                    },
                ]
            })
            .collect(),
    );
    let mut hashes = vec![];
    for (index, events) in packets.into_iter().enumerate() {
        input.advance_validated(&events, DT, &actions).unwrap();
        for world in [&mut a, &mut b, &mut expected] {
            world.step().unwrap();
        }
        native.tick_with_input(&mut a, DT, &input).unwrap();
        dev.tick_with_input(&mut b, DT, &input).unwrap();
        assert_eq!(a.inspect(StableId([1; 16])), b.inspect(StableId([1; 16])));
        let mut cap = Capability::default();
        let epoch = cap.begin_with_input(&mut expected, DT, &input).unwrap();
        let mut bytes = vec![0; MAX_INPUT_BYTES];
        let len = cap
            .invoke(
                &mut bytes,
                epoch,
                Call::ReadInput {
                    ptr: 0,
                    capacity: MAX_INPUT_BYTES as i32,
                },
            )
            .unwrap() as usize;
        let hash = bytes[..len].iter().fold(2166136261u32, |h, b| {
            (h ^ u32::from(*b)).wrapping_mul(16777619)
        });
        assert_eq!(
            a.inspect(StableId([1; 16])).unwrap().transform.translation,
            [f64::from(hash), len as f64, (index + 1) as f64]
        );
        hashes.push(hash);
    }
    assert!(hashes.windows(2).all(|pair| pair[0] != pair[1]));
}
#[test]
fn both_hosts_reject_missing_input_and_bad_reads_after_staging_without_publication() {
    for (name, source) in sources().into_iter().take(4) {
        let mut a = world();
        let mut b = world();
        let before = a.inspect(StableId([1; 16]));
        let mut native = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&wat::parse_str(source).unwrap()).unwrap();
        let input = InputRuntime::default();
        if name == "input" {
            assert!(native.tick(&mut a, DT).is_err());
            assert!(dev.tick(&mut b, DT).is_err());
        } else {
            assert!(native.tick_with_input(&mut a, DT, &input).is_err());
            assert!(dev.tick_with_input(&mut b, DT, &input).is_err());
        }
        assert!(native.is_failed());
        assert!(dev.is_failed());
        assert_eq!(a.inspect(StableId([1; 16])), before);
        assert_eq!(b.inspect(StableId([1; 16])), before);
        assert!(native.tick_with_input(&mut a, DT, &input).is_err());
        assert!(dev.tick_with_input(&mut b, DT, &input).is_err());
    }
}
#[test]
fn input_namespace_has_no_startup_authority_and_unknown_version_is_rejected() {
    let (_, source) = sources()[4];
    assert!(StaticModule::new(embedded::linked("input-start").unwrap()).is_err());
    assert!(DevModuleHost::new(&wat::parse_str(source).unwrap()).is_err());
    let unsupported = sources()[0].1.replace("incant_input_v1", "incant_input_v2");
    assert!(DevModuleHost::new(&wat::parse_str(&unsupported).unwrap()).is_err());
}
