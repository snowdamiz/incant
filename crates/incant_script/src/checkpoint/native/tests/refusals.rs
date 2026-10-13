use super::*;
use incant_runtime::{CookedAudio, CookedUi, FrameCommands, StructuralCommand};
use incant_types::{UiKind, UiNode};
use std::sync::Arc;

#[test]
fn failed_sessions_and_missing_opt_in_never_create_or_restore_checkpoints() {
    let mut good = session("");
    let checkpoint = capture(&mut good).unwrap();
    let mut failed = session("throw Error('failed');");
    assert!(failed.tick(&[]).is_err());
    assert!(matches!(
        capture(&mut failed),
        Err(CheckpointError::FailedSession)
    ));
    assert!(matches!(
        restore(&mut failed, &checkpoint),
        Err(CheckpointError::FailedSession)
    ));
    let code = CompiledScript::compile("exports.default={update(){}};").unwrap();
    let mut unopted = NativeSession::new(scene(), code, Default::default()).unwrap();
    assert!(matches!(
        capture(&mut unopted),
        Err(CheckpointError::Unsupported(_))
    ));
    let (initial, code) = super::super::super::tests::grouped(false);
    let mut partly_opted = NativeSession::new(initial, code, Default::default()).unwrap();
    assert!(matches!(
        capture(&mut partly_opted),
        Err(CheckpointError::Unsupported(_))
    ));
}

#[test]
fn incompatible_code_world_and_late_vm_initialization_fail_atomically() {
    let mut source = session("t.chunks[0].transforms[0]+=1;");
    source.tick(&[]).unwrap();
    let mut checkpoint = capture(&mut source).unwrap();
    let mut other = session("t.chunks[0].transforms[0]+=2;");
    let before = bytes(&other);
    assert!(matches!(
        restore(&mut other, &checkpoint),
        Err(CheckpointError::ProgramMismatch)
    ));
    assert_eq!(bytes(&other), before);
    let initial = CookedScene::new(id(98), 64, scene().entities().to_vec()).unwrap();
    let mut other =
        NativeSession::new(initial, checkpoint.code.clone(), Default::default()).unwrap();
    let before = bytes(&other);
    assert!(restore(&mut other, &checkpoint).is_err());
    assert_eq!(bytes(&other), before);

    // Test-only corruption reaches the late stage without adding a public code
    // setter/importer: the old live VM remains runnable while candidate init traps.
    let throwing = CompiledScript::compile("throw Error('candidate init');").unwrap();
    checkpoint.code = throwing.clone();
    match &mut source.host {
        SessionVm::Bytecode(host) => host._compiled = throwing,
        #[cfg(feature = "wasmtime-host")]
        _ => unreachable!(),
        #[cfg(all(feature = "static-module", not(feature = "wasmtime-host")))]
        _ => unreachable!(),
    }
    let before = source.world.snapshot();
    let incarnation = source.world.incarnation();
    assert!(matches!(
        restore(&mut source, &checkpoint),
        Err(CheckpointError::Script(_))
    ));
    assert_eq!(source.world.snapshot(), before);
    assert_eq!(source.world.incarnation(), incarnation);
    assert_eq!(host(&source).unwrap().schedule.tick, 1);
    source.tick(&[]).unwrap();
    assert_eq!(
        source.world.inspect(id(1)).unwrap().transform.translation[0],
        2.
    );
}

fn ui_scene() -> CookedScene {
    scene()
        .with_ui(vec![CookedUi {
            entity: id(1),
            node: Arc::new(UiNode {
                kind: UiKind::Root,
                style: Default::default(),
                order: 0,
                disabled: false,
            }),
        }])
        .unwrap()
}
fn audio_scene() -> CookedScene {
    scene()
        .with_audio(vec![CookedAudio {
            entity: id(1),
            bus: None,
            source: None,
            listener: true,
        }])
        .unwrap()
}
#[test]
fn hosted_owners_cannot_disappear_after_declarations_are_removed() {
    for (initial, message) in [
        (ui_scene(), "hosted UI state is required"),
        (audio_scene(), "retained audio owner is required"),
    ] {
        let mut live = NativeSession::new(initial, program(""), Default::default()).unwrap();
        assert!(matches!(capture(&mut live), Err(CheckpointError::Unsupported(s)) if s == message));
        let mut commands = FrameCommands::default();
        commands.push(StructuralCommand::Despawn(id(1))).unwrap();
        live.world.apply(&mut commands).unwrap();
        assert!(matches!(capture(&mut live), Err(CheckpointError::Unsupported(s)) if s == message));
        // Even a matching script checkpoint cannot rewind away the receiver's
        // host obligations. The check precedes world reconstruction/VM creation.
        let mut plain = session("");
        let checkpoint = capture(&mut plain).unwrap();
        assert!(
            matches!(restore(&mut live, &checkpoint), Err(CheckpointError::Unsupported(s)) if s == message)
        );
    }
}

#[test]
fn captured_entry_values_are_immutable_after_live_state_changes() {
    let mut live = session("t.state.set({n:t.clock.tick});");
    live.tick(&[]).unwrap();
    let checkpoint = capture(&mut live).unwrap();
    for _ in 0..5 {
        live.tick(&[]).unwrap();
    }
    let restored = reconstruct(&checkpoint).unwrap();
    assert_eq!(host(&restored).unwrap().schedule.tick, 1);
    assert_eq!(
        host(&restored)
            .unwrap()
            .persistent
            .borrow()
            .entries
            .values()
            .next()
            .unwrap()
            .value,
        crate::ScriptValue::Object(std::collections::BTreeMap::from([(
            "n".into(),
            crate::ScriptValue::Number(1.)
        )]))
    );
    assert_eq!(host(&live).unwrap().schedule.tick, 6);
}
