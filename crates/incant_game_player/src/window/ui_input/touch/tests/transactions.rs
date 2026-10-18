use super::*;

#[test]
fn surface_cancellation_refusal_preserves_all_captures_for_retry() {
    let ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        1,
        NativePhase::Started,
        [10., 10.],
    );
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        2,
        NativePhase::Started,
        [350., 80.],
    );
    input.accepted();
    physical.clear();
    for _ in 0..MAX_EVENTS_PER_TICK {
        physical
            .push(InputEvent::Key {
                code: incant_input::KeyCode::Space,
                down: true,
            })
            .unwrap();
    }
    let before = physical.events().to_vec();
    assert!(matches!(
        input.defer_surface(Some(&ui), &mut physical),
        Err(Error::InputCapacity)
    ));
    assert_eq!(physical.events(), before);
    assert!(input.receipts().is_empty() && !input.awaiting_surface());
    physical.clear();
    input.defer_surface(Some(&ui), &mut physical).unwrap();
    assert!(matches!(
        physical.events(),
        [InputEvent::Touch {
            id: 2,
            phase: TouchPhase::Cancel,
            ..
        }]
    ));
    assert!(matches!(input.receipts(), [UiEvent::CancelPointer { .. }]));
    assert!(input.touches.slots.iter().all(Option::is_none));
}

#[test]
fn invalid_touch_and_full_queue_roll_back_contact_allocator_and_discrete_order() {
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    for _ in 0..MAX_EVENTS_PER_TICK {
        physical
            .push(InputEvent::Key {
                code: incant_input::KeyCode::Space,
                down: true,
            })
            .unwrap();
    }
    let before = physical.events().to_vec();
    assert!(matches!(
        input.window(
            None,
            &mut physical,
            &native(1, NativePhase::Started, [0., 0.]),
            1.
        ),
        Err(Error::InputCapacity)
    ));
    assert_eq!(physical.events(), before);
    assert!(input.touches.slots.iter().all(Option::is_none));
    assert_eq!(input.touches.next_id, 0);
    physical.clear();
    deliver(
        &mut input,
        &mut physical,
        None,
        1,
        NativePhase::Started,
        [1., 2.],
    );
    let before = physical.events().to_vec();
    for (p, scale) in [
        ([f64::NAN, 0.], 1.),
        ([2_000_001., 0.], 2.),
        ([1., 2.], 0.),
        ([1., 2.], f64::INFINITY),
    ] {
        assert!(
            input
                .window(
                    None,
                    &mut physical,
                    &native(1, NativePhase::Ended, p),
                    scale
                )
                .is_err()
        );
        assert_eq!(physical.events(), before);
    }
    assert!(matches!(
        input.window(
            None,
            &mut physical,
            &native(1, NativePhase::Started, [1., 2.]),
            1.
        ),
        Err(Error::TouchSequence)
    ));
    assert_eq!(physical.events(), before);
    deliver(
        &mut input,
        &mut physical,
        None,
        1,
        NativePhase::Ended,
        [1., 2.],
    );
    let mut state = InputRuntime::default();
    state.advance(physical.events(), 1. / 60.).unwrap();
    assert!(matches!(
        state.frame().gestures.as_slice(),
        [Gesture::Tap { id: 1, .. }]
    ));
}

#[test]
fn ui_capacity_refusal_rolls_back_simultaneous_owner_fence_cancellations() {
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    deliver(
        &mut input,
        &mut physical,
        None,
        1,
        NativePhase::Started,
        [350., 80.],
    );
    let mut state = InputRuntime::default();
    state.advance(physical.events(), 1. / 60.).unwrap();
    physical.clear();
    let ui = runtime(true);
    for _ in 0..MAX_UI_EVENTS {
        input.push(UiEvent::Focus(id(2)), 0).unwrap();
    }
    assert!(matches!(
        input.window(
            Some(&ui),
            &mut physical,
            &native(2, NativePhase::Started, [10., 10.]),
            1.
        ),
        Err(Error::UiCapacity)
    ));
    assert!(physical.events().is_empty());
    assert_eq!(input.receipts().len(), MAX_UI_EVENTS);
    assert!(input.observed.is_none());
    assert_eq!(input.touches.next_id, 1);
    input.accepted();
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        2,
        NativePhase::Started,
        [10., 10.],
    );
    assert!(matches!(
        physical.events(),
        [InputEvent::Touch {
            id: 1,
            phase: TouchPhase::Cancel,
            ..
        }]
    ));
    state.advance(physical.events(), 1. / 60.).unwrap();
    assert!(state.frame().touches.is_empty());
    assert_eq!(input.receipts().len(), 1);
}

#[test]
fn refused_ui_candidate_retains_exact_touch_receipts_until_acceptance() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        1,
        NativePhase::Started,
        [10., 10.],
    );
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        1,
        NativePhase::Ended,
        [10., 10.],
    );
    assert!(
        ui.prepare(input.receipts(), [f32::NAN, 100.], &mut Measure)
            .is_err()
    );
    assert!(ui.actions().is_empty());
    assert_eq!(input.receipts().len(), 2);
    accept(&mut ui, &mut input);
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    accept(&mut ui, &mut input);
    assert!(ui.actions().is_empty());
    assert!(physical.events().is_empty());
}

#[test]
fn rejected_tick_preserves_touch_vm_world_and_pcm_then_same_packet_retries() {
    use incant_runtime::{CookedEntity, CookedScene};
    use incant_script::CompiledScript;
    let scene = CookedScene::new(
        id(99),
        60,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap();
    let code = CompiledScript::compile("let calls=0; exports.default={update({chunks,input}){calls++;chunks[0].transforms[0]=calls; chunks[0].transforms[1]+=input.actions.gesture0.pressed?1:0;}};").unwrap();
    let mut game =
        incant_game_runtime::OfflineGameSession::new(scene, code, actions(), vec![], 48000)
            .unwrap();
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    deliver(
        &mut input,
        &mut physical,
        None,
        9,
        NativePhase::Started,
        [0., 0.],
    );
    let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
    game.tick(physical.events(), &mut pcm).unwrap();
    physical.clear();
    deliver(
        &mut input,
        &mut physical,
        None,
        9,
        NativePhase::Ended,
        [1., 1.],
    );
    let before = game.world().unwrap().snapshot();
    let frame = game.input().unwrap().clone();
    let mut invalid = physical.events().to_vec();
    invalid.push(InputEvent::Touch {
        id: 999,
        phase: TouchPhase::Move,
        position: [0., 0.],
    });
    pcm.fill(0.7);
    assert!(game.tick(&invalid, &mut pcm).is_err());
    assert_eq!(game.world().unwrap().snapshot(), before);
    assert_eq!(game.input().unwrap(), &frame);
    assert_eq!(game.rendered_frames().unwrap(), 800);
    assert!(pcm.iter().all(|v| *v == 0.7));
    game.tick(physical.events(), &mut pcm).unwrap();
    assert_eq!(
        game.world()
            .unwrap()
            .inspect(id(1))
            .unwrap()
            .transform
            .translation,
        [2., 1., 0.]
    );
    assert_eq!(game.rendered_frames().unwrap(), 1600);
    assert!(pcm.iter().all(|v| *v == 0.));
}
