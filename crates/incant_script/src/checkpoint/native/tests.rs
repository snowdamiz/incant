use super::*;
use incant_input::{InputEvent, KeyCode};
use incant_runtime::{CookedEntity, CookedScene};

#[cfg(any(feature = "ui", feature = "audio"))]
mod hosted;
mod physics;
mod refusals;

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn payload() -> CheckpointPayloadBudget {
    CheckpointPayloadBudget::new(64 * 1024 * 1024).unwrap()
}
fn planar() -> PlanarCheckpointWorkBudget {
    PlanarCheckpointWorkBudget::new(100_000_000)
}
fn spatial() -> SpatialCheckpointWorkBudget {
    SpatialCheckpointWorkBudget::new(100_000_000)
}
fn allocations() -> CheckpointAllocationBudget {
    CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000)
}
fn capture(s: &mut NativeSession) -> Result<NativeSessionCheckpoint> {
    s.checkpoint(&mut payload(), &mut planar(), &mut spatial())
}
fn reconstruct(c: &NativeSessionCheckpoint) -> Result<NativeSession> {
    c.reconstruct(
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut allocations(),
    )
}
fn restore(s: &mut NativeSession, c: &NativeSessionCheckpoint) -> Result<()> {
    s.restore_checkpoint(
        c,
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut allocations(),
    )
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        64,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap()
}
fn program(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default={{persistence:{{version:1}},update(t){{{body}}}}};"
    ))
    .unwrap()
}
fn session(body: &str) -> NativeSession {
    NativeSession::new(scene(), program(body), Default::default()).unwrap()
}
fn bytes(s: &NativeSession) -> Vec<u8> {
    s.checkpoint_bytes().unwrap()
}

#[test]
fn neutral_input_captured_bindings_and_restart_authority_are_explicit() {
    use incant_input::{InputAction, InputBinding};
    let actions = InputActions::from([(
        "jump".into(),
        InputAction::Button {
            bindings: vec![InputBinding::Key {
                code: KeyCode::Space,
                scale: [1., 0.],
            }],
            threshold: 0.5,
        },
    )]);
    let mut live = session("if(t.clock.tick===2)t.commands.requestRestart();");
    live.rebind(actions.clone()).unwrap();
    live.enable_restart_requests().unwrap();
    live.tick(&[InputEvent::Key {
        code: KeyCode::Space,
        down: true,
    }])
    .unwrap();
    let checkpoint = capture(&mut live).unwrap();
    let fresh = reconstruct(&checkpoint).unwrap();
    assert!(!host(&fresh).unwrap().restart_requests);
    assert_eq!(fresh.bindings.definitions(), &actions);
    assert!(fresh.input().unwrap().keyboard.held.is_empty());
    live.rebind(Default::default()).unwrap();
    let incarnation = live.world.incarnation();
    restore(&mut live, &checkpoint).unwrap();
    assert_ne!(live.world.incarnation(), incarnation);
    assert_eq!(live.bindings.definitions(), &actions);
    assert!(host(&live).unwrap().restart_requests);
    let frame = live.input().unwrap();
    assert!(frame.keyboard.held.is_empty());
    assert!(frame.keyboard.pressed.is_empty());
    assert!(frame.keyboard.released.is_empty());
    live.tick(&[]).unwrap();
    assert!(live.input().unwrap().keyboard.released.is_empty());
    assert!(matches!(
        capture(&mut live),
        Err(CheckpointError::Unsupported(
            "pending session restart request"
        ))
    ));
    let before = live.world.snapshot();
    assert!(matches!(
        restore(&mut live, &checkpoint),
        Err(CheckpointError::Unsupported(
            "pending session restart request"
        ))
    ));
    assert_eq!(live.world.snapshot(), before);
    assert_eq!(
        live.take_session_request().unwrap(),
        Some(crate::SessionRequest::Restart)
    );
    assert!(capture(&mut live).is_ok());
}

#[test]
fn grouped_retirement_does_not_reattach_reused_stable_ids() {
    use incant_runtime::{FrameCommands, StructuralCommand};
    let (initial, code) = super::super::tests::grouped(true);
    let mut live = NativeSession::new(initial.clone(), code, Default::default()).unwrap();
    for _ in 0..4 {
        live.tick(&[]).unwrap();
    }
    let checkpoint = capture(&mut live).unwrap();
    let mut restored = reconstruct(&checkpoint).unwrap();
    for _ in 0..16 {
        live.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        assert_eq!(bytes(&live), bytes(&restored));
    }
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(2))).unwrap();
    live.world.apply(&mut commands).unwrap();
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::Spawn(initial.entities()[1].clone()))
        .unwrap();
    live.world.apply(&mut commands).unwrap();
    let checkpoint = capture(&mut live).unwrap();
    assert!(!checkpoint.state.active.contains(&id(2)));
    assert!(
        checkpoint
            .state
            .store
            .entries
            .keys()
            .all(|k| k.entity != Some(id(2)))
    );
    let mut restored = reconstruct(&checkpoint).unwrap();
    for _ in 0..16 {
        live.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        assert_eq!(bytes(&live), bytes(&restored));
    }
}

#[test]
fn fresh_vm_reinitializes_globals_but_keeps_timers_and_explicit_state() {
    let code = CompiledScript::compile(
        r#"
        let ephemeral=0;
        exports.default={persistence:{version:1},update(t){
            ephemeral++;
            let s=t.state.get() || {count:0,events:0};
            s={count:s.count+1,events:s.events+t.timers.events.length};
            if(t.clock.tick===1)t.timers.set('beat',3,3);
            t.state.set(s);
            t.chunks[0].transforms[0]=s.count*2+s.events;
            t.chunks[0].transforms[1]=ephemeral;
        }};
    "#,
    )
    .unwrap();
    let mut live = NativeSession::new(scene(), code, Default::default()).unwrap();
    for _ in 0..5 {
        live.tick(&[]).unwrap();
    }
    let mut restored = reconstruct(&capture(&mut live).unwrap()).unwrap();
    for i in 1..=20 {
        live.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        let a = live.world.inspect(id(1)).unwrap().transform.translation;
        let b = restored.world.inspect(id(1)).unwrap().transform.translation;
        assert_eq!(a[0], b[0]);
        assert_eq!(a[1], f64::from(i + 5));
        assert_eq!(b[1], f64::from(i));
        let a = host(&live).unwrap();
        let b = host(&restored).unwrap();
        assert_eq!(a.schedule.tick, b.schedule.tick);
        assert_eq!(
            a.schedule.elapsed_seconds.to_bits(),
            b.schedule.elapsed_seconds.to_bits()
        );
    }
}
