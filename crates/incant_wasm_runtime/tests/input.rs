use incant_input::*;
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_wasm_runtime::{Call, Capability, INPUT_MAGIC, MAX_INPUT_BYTES};
const DT: f64 = 1. / 64.;
fn snapshot(input: &InputRuntime, tick: usize) -> Vec<u8> {
    let mut world = NativeWorld::new(CookedScene::new(StableId([0; 16]), 64, vec![]).unwrap());
    for _ in 0..tick {
        world.step().unwrap();
    }
    let mut cap = Capability::default();
    let epoch = cap.begin_with_input(&mut world, DT, input).unwrap();
    let mut memory = vec![0xa5; MAX_INPUT_BYTES + 16];
    let len = cap.invoke(&mut memory, epoch, Call::InputLen).unwrap();
    assert_eq!(
        cap.invoke(
            &mut memory,
            epoch,
            Call::ReadInput {
                ptr: 8,
                capacity: len
            }
        )
        .unwrap(),
        len
    );
    assert!(
        memory[..8]
            .iter()
            .chain(&memory[8 + len as usize..])
            .all(|b| *b == 0xa5)
    );
    cap.finish(&mut world).unwrap();
    assert!(cap.invoke(&mut memory, epoch, Call::InputLen).is_err());
    memory[8..8 + len as usize].to_vec()
}
struct Reader<'a>(&'a [u8]);
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let (value, rest) = self.0.split_at(N);
        self.0 = rest;
        value.try_into().unwrap()
    }
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.take())
    }
    fn f64(&mut self) -> f64 {
        f64::from_le_bytes(self.take())
    }
    fn point(&mut self) -> [f64; 2] {
        [self.f64(), self.f64()]
    }
}
fn touch(id: u32, phase: TouchPhase, position: [f64; 2]) -> InputEvent {
    InputEvent::Touch {
        id,
        phase,
        position,
    }
}
#[test]
fn wire_snapshot_preserves_raw_controls_named_edges_and_tap_with_explicit_tags() {
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
    let mut input = InputRuntime::default();
    input
        .advance_validated(
            &[
                InputEvent::Key {
                    code: KeyCode::Space,
                    down: true,
                },
                InputEvent::Key {
                    code: KeyCode::Space,
                    down: false,
                },
                InputEvent::Key {
                    code: KeyCode::F12,
                    down: true,
                },
                InputEvent::MouseButton {
                    button: MouseButton::Forward,
                    down: true,
                },
                InputEvent::PointerPosition { position: [3., 4.] },
                InputEvent::PointerMotion { delta: [-2., 1.] },
                InputEvent::Wheel {
                    delta: [0., -3.],
                    unit: WheelUnit::Lines,
                },
                InputEvent::GamepadConnected { id: 7 },
                InputEvent::GamepadButton {
                    id: 7,
                    button: GamepadButton::RightTrigger,
                    value: 0.75,
                },
                InputEvent::GamepadAxis {
                    id: 7,
                    axis: GamepadAxis::LeftX,
                    value: -0.5,
                },
                touch(4, TouchPhase::Down, [10., 20.]),
                touch(4, TouchPhase::Up, [11., 21.]),
            ],
            DT,
            &actions,
        )
        .unwrap();
    let bytes = snapshot(&input, 3);
    let mut r = Reader(&bytes);
    assert_eq!(&r.take::<8>(), INPUT_MAGIC);
    assert_eq!(r.u32() as usize, bytes.len());
    assert_eq!(r.u32(), 1);
    assert_eq!(u64::from_le_bytes(r.take()), 3);
    assert_eq!(r.u32(), 1);
    assert_eq!(r.take::<4>(), [4, 0, 6, 0]);
    assert_eq!(r.point(), [0., 0.]);
    assert_eq!(&r.take::<4>(), b"jump");
    // F12 has stable tag 103; Space has stable tag 40. Sub-tick tap keeps both edges.
    assert_eq!(u128::from_le_bytes(r.take()), 1u128 << 103);
    assert_eq!(
        u128::from_le_bytes(r.take()),
        (1u128 << 103) | (1u128 << 40)
    );
    assert_eq!(u128::from_le_bytes(r.take()), 1u128 << 40);
    assert_eq!(r.take::<4>(), [16, 16, 0, 1]);
    assert_eq!(r.point(), [3., 4.]);
    assert_eq!(r.point(), [-2., 1.]);
    assert_eq!(r.point(), [0., 0.]);
    assert_eq!(r.point(), [0., -3.]);
    assert_eq!(r.u32(), 1);
    assert_eq!(r.u32(), 7);
    assert_eq!(r.u32(), 1 << 7);
    assert_eq!(r.u32(), 1 << 7);
    assert_eq!(r.u32(), 0);
    assert_eq!(r.u32(), 1 << 7);
    for i in 0..17 {
        assert_eq!(r.f64(), if i == 7 { 0.75 } else { 0. });
    }
    assert_eq!(r.point(), [-0.5, 0.]);
    assert_eq!(r.point(), [0., 0.]);
    assert_eq!(r.u32(), 1);
    assert_eq!(r.u32(), 7);
    assert_eq!(r.u32(), 0);
    assert_eq!(r.u32(), 0);
    assert_eq!(r.u32(), 2);
    for (phase, point) in [(0, [10., 20.]), (2, [11., 21.])] {
        assert_eq!(r.u32(), 4);
        assert_eq!(r.take::<4>(), [phase, 0, 0, 0]);
        assert_eq!(r.point(), point);
    }
    assert_eq!(r.u32(), 1);
    assert_eq!(r.take::<4>(), [1, 0, 0, 0]);
    assert_eq!(r.u32(), 4);
    assert_eq!(r.point(), [11., 21.]);
    assert!(r.0.is_empty());
    let first = bytes;
    input.advance_validated(&[], DT, &actions).unwrap();
    assert_ne!(snapshot(&input, 3), first, "edges are per accepted tick");
}
#[test]
fn maximum_accepted_touch_packet_and_actions_fit_fixed_snapshot_cap() {
    let actions = ValidatedActions::new(
        (0..64)
            .map(|n| {
                (
                    format!("action{n:058}"),
                    InputAction::Button {
                        bindings: vec![],
                        threshold: 0.5,
                    },
                )
            })
            .collect(),
    )
    .unwrap();
    let mut input = InputRuntime::default();
    let events: Vec<_> = (0..512)
        .flat_map(|id| {
            [
                touch(id, TouchPhase::Down, [0., 0.]),
                touch(id, TouchPhase::Up, [0., 0.]),
            ]
        })
        .collect();
    input.advance_validated(&events, DT, &actions).unwrap();
    assert_eq!(input.frame().touch_changes.len(), 1024);
    assert_eq!(input.frame().gestures.len(), 512);
    let bytes = snapshot(&input, 1);
    assert!(bytes.len() < MAX_INPUT_BYTES);
    assert!(bytes.len() > 40_000);
}
#[test]
fn invalid_destinations_epochs_and_call_exhaustion_never_partially_copy() {
    let mut world = NativeWorld::new(CookedScene::new(StableId([0; 16]), 64, vec![]).unwrap());
    let input = InputRuntime::default();
    let mut cap = Capability::default();
    let epoch = cap.begin_with_input(&mut world, DT, &input).unwrap();
    let mut memory = vec![0xa5; 512];
    let len = cap.invoke(&mut memory, epoch, Call::InputLen).unwrap();
    for (ptr, capacity) in [
        (-1, len),
        (0, len - 1),
        (MAX_INPUT_BYTES as i32 - 1, len),
        (0, -1),
        (0, i32::MAX),
    ] {
        assert!(
            cap.invoke(&mut memory, epoch, Call::ReadInput { ptr, capacity })
                .is_err()
        );
        assert!(memory.iter().all(|b| *b == 0xa5));
    }
    assert!(cap.invoke(&mut memory, epoch - 1, Call::InputLen).is_err());
    let mut copied = 0;
    loop {
        memory.fill(0xa5);
        match cap.invoke(
            &mut memory,
            epoch,
            Call::ReadInput {
                ptr: 0,
                capacity: len,
            },
        ) {
            Ok(_) => copied += len as usize,
            Err(error) => {
                assert!(error.to_string().contains("quota"));
                assert!(memory.iter().all(|b| *b == 0xa5));
                break;
            }
        }
    }
    assert!(copied > 0);
    cap.abort();
    let mut cap = Capability::default();
    let epoch = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(&mut memory, epoch, Call::InputLen)
            .unwrap_err()
            .to_string()
            .contains("unavailable")
    );
}
fn gesture_reader(bytes: &[u8]) -> Reader<'_> {
    let mut r = Reader(&bytes[24..]);
    assert_eq!(r.u32(), 0);
    r.0 = &r.0[48 + 68..]; // keyboard and pointer
    for _ in 0..3 {
        assert_eq!(r.u32(), 0);
    } // pads and lifecycle IDs
    let touches = r.u32() as usize;
    r.0 = &r.0[touches * 36..];
    let changes = r.u32() as usize;
    r.0 = &r.0[changes * 24..];
    r
}
#[test]
fn long_press_swipe_and_pinch_keep_their_typed_numeric_payloads() {
    let mut input = InputRuntime::default();
    input
        .advance(&[touch(7, TouchPhase::Down, [3., 4.])], DT)
        .unwrap();
    for _ in 0..32 {
        input.advance(&[], DT).unwrap();
    }
    let bytes = snapshot(&input, 33);
    let mut r = gesture_reader(&bytes);
    assert_eq!(r.u32(), 1);
    assert_eq!(r.take::<4>(), [2, 0, 0, 0]);
    assert_eq!(r.u32(), 7);
    assert_eq!(r.point(), [3., 4.]);
    assert!(r.0.is_empty());
    let mut input = InputRuntime::default();
    input
        .advance(&[touch(7, TouchPhase::Down, [0., 0.])], DT)
        .unwrap();
    input
        .advance(&[touch(7, TouchPhase::Move, [30., 40.])], DT)
        .unwrap();
    input
        .advance(&[touch(7, TouchPhase::Up, [30., 40.])], DT)
        .unwrap();
    let bytes = snapshot(&input, 3);
    let mut r = gesture_reader(&bytes);
    assert_eq!(r.u32(), 1);
    assert_eq!(r.take::<4>(), [3, 0, 0, 0]);
    assert_eq!(r.u32(), 7);
    assert_eq!(r.point(), [0.6, 0.8]);
    assert_eq!(r.f64(), 50.);
    assert_eq!(r.f64(), 2. * DT);
    assert!(r.0.is_empty());
    let mut input = InputRuntime::default();
    input
        .advance(
            &[
                touch(9, TouchPhase::Down, [10., 0.]),
                touch(3, TouchPhase::Down, [0., 0.]),
            ],
            DT,
        )
        .unwrap();
    input
        .advance(&[touch(9, TouchPhase::Move, [0., 20.])], DT)
        .unwrap();
    let bytes = snapshot(&input, 2);
    let mut r = gesture_reader(&bytes);
    assert_eq!(r.u32(), 1);
    assert_eq!(r.take::<4>(), [4, 0, 0, 0]);
    assert_eq!([r.u32(), r.u32()], [3, 9]);
    assert_eq!(r.point(), [0., 10.]);
    assert_eq!(r.f64(), 2.);
    assert_eq!(r.f64(), std::f64::consts::FRAC_PI_2);
    assert!(r.0.is_empty());
}
#[test]
fn large_snapshot_copies_share_numeric_transfer_quota_and_last_failure_is_atomic() {
    let mut world = NativeWorld::new(CookedScene::new(StableId([0; 16]), 64, vec![]).unwrap());
    let mut input = InputRuntime::default();
    let events: Vec<_> = (0..512)
        .flat_map(|id| {
            [
                touch(id, TouchPhase::Down, [0., 0.]),
                touch(id, TouchPhase::Up, [0., 0.]),
            ]
        })
        .collect();
    input.advance(&events, DT).unwrap();
    let mut cap = Capability::default();
    let epoch = cap.begin_with_input(&mut world, DT, &input).unwrap();
    let mut memory = vec![0; MAX_INPUT_BYTES];
    let len = cap.invoke(&mut memory, epoch, Call::InputLen).unwrap();
    let mut total = 0;
    loop {
        memory.fill(0xa5);
        match cap.invoke(
            &mut memory,
            epoch,
            Call::ReadInput {
                ptr: 0,
                capacity: len,
            },
        ) {
            Ok(_) => total += len as usize,
            Err(error) => {
                assert!(error.to_string().contains("transfer quota"));
                assert!(memory.iter().all(|b| *b == 0xa5));
                break;
            }
        }
    }
    assert!(total <= 128 * 1024 * 1024 && total + len as usize > 128 * 1024 * 1024);
    cap.abort();
}
