use super::*;
use crate::tests::{check, id, world};
use incant_input::haptics::{HapticCapabilities, HapticEffect, HapticError, HapticTarget};

fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default={{persistence:{{version:1}},update({{chunks,commands,clock}}){{{body}}}}};"
    ))
    .unwrap()
}
fn capabilities() -> HapticCapabilities {
    HapticCapabilities::default().with_gamepad(0, 7).unwrap()
}
const PLAY: &str = "commands.playHaptic(commands.hapticCapabilities()[0],{kind:'rumble',strong:0.5,weak:1,durationMs:125})";
#[test]
fn haptic_output_commits_only_with_the_complete_transaction() {
    for suffix in [
        "throw Error('late')",
        "commands.despawn('00000000000000000000000999')",
        "chunks[0].transforms[0]=NaN",
    ] {
        let mut host =
            ScriptHost::new(code(&format!("chunks[0].transforms[0]=77;{PLAY};{suffix}"))).unwrap();
        host.set_haptic_capabilities(capabilities()).unwrap();
        let mut world = world();
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.1).is_err());
        assert_eq!(world.snapshot(), before);
        assert!(matches!(
            host.take_haptics(),
            Err(ScriptError::FailedSession)
        ));
    }
}
#[test]
fn haptic_invalid_caught_calls_poison_numeric_prefix() {
    for request in [
        "commands.playHaptic({kind:'gamepad',id:0,generation:6},{kind:'rumble',strong:1,weak:0,durationMs:1})",
        "commands.playHaptic(commands.hapticCapabilities()[0],{kind:'rumble',strong:NaN,weak:0,durationMs:1})",
        "commands.playHaptic(commands.hapticCapabilities()[0],{kind:'rumble',strong:1.000000001,weak:0,durationMs:1})",
        "commands.playHaptic(commands.hapticCapabilities()[0],{kind:'rumble',strong:1,weak:0,durationMs:0})",
        "commands.playHaptic(commands.hapticCapabilities()[0],{kind:'impact',intensity:1})",
        "commands.hapticCapabilities(1)",
        "commands.stopHaptic({kind:'mystery',generation:7})",
    ] {
        let mut host = ScriptHost::new(code(&format!(
            "chunks[0].transforms[0]=77;try{{{request}}}catch(_){{}}"
        )))
        .unwrap();
        host.set_haptic_capabilities(capabilities()).unwrap();
        let mut world = world();
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.1).is_err(), "{request}");
        assert_eq!(world.snapshot(), before);
    }
}
#[test]
fn haptic_queue_is_bounded_and_expired_functions_cannot_actuate() {
    let mut host=ScriptHost::new(code(&format!("if(clock.tick===1){{globalThis.old=commands.stopHaptic;for(let i=0;i<32;i++){{{PLAY}}}}}else{{try{{old(commands.hapticCapabilities()[0])}}catch(_){{}}}}"))).unwrap();
    host.set_haptic_capabilities(capabilities()).unwrap();
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::Haptics(HapticError::Pending))
    ));
    let output = host.take_haptics().unwrap();
    assert_eq!(output.len(), 32);
    assert_eq!(
        output.iter().next().unwrap().target,
        HapticTarget::Gamepad {
            id: 0,
            generation: 7
        }
    );
    assert_eq!(
        output.iter().next().unwrap().effect,
        HapticEffect::Rumble {
            strong: 0.5,
            weak: 1.,
            duration_ms: 125
        }
    );
    check(
        &host,
        "let refused=false;try{old({kind:'gamepad',id:0,generation:7})}catch(_){refused=true}if(!refused)throw Error('outside tick')",
    );
    host.tick(&mut world, 0.1).unwrap();
    assert!(host.take_haptics().unwrap().is_empty());
    let mut host = ScriptHost::new(code(&format!(
        "for(let i=0;i<33;i++){{try{{{PLAY}}}catch(_){{}}}}"
    )))
    .unwrap();
    host.set_haptic_capabilities(capabilities()).unwrap();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::Haptics(HapticError::Capacity))
    ));
}
#[test]
fn haptic_reentrant_property_failure_cannot_commit_outer_request() {
    let mut host=ScriptHost::new(code("const t=commands.hapticCapabilities()[0];commands.playHaptic(t,{kind:'rumble',get strong(){try{commands.stopHaptic({kind:'gamepad',id:0,generation:6})}catch(_){}return 1},weak:0,durationMs:50})")).unwrap();
    host.set_haptic_capabilities(capabilities()).unwrap();
    assert!(host.tick(&mut world(), 0.1).is_err());
    assert!(matches!(
        host.take_haptics(),
        Err(ScriptError::FailedSession)
    ));
}
#[test]
fn haptic_checkpoint_restores_neutral_capabilities_and_never_replays_output() {
    let scene = incant_runtime::CookedScene::new(id(99), 60, vec![]).unwrap();
    let mut session = NativeSession::new(scene.clone(), code(PLAY), Default::default()).unwrap();
    session.set_haptic_capabilities(capabilities()).unwrap();
    session.tick(&[]).unwrap();
    let before = session.world().unwrap().tick();
    assert!(matches!(
        session.tick(&[]),
        Err(SessionError::Script(ScriptError::Haptics(
            HapticError::Pending
        )))
    ));
    assert_eq!(session.world().unwrap().tick(), before);
    assert!(matches!(
        session.checkpoint_bytes(),
        Err(CheckpointError::Unsupported("pending haptic output"))
    ));
    session.take_haptics().unwrap();
    let bytes = session.checkpoint_bytes().unwrap();
    let mut restored =
        NativeSession::from_checkpoint_bytes(&bytes, &scene, code(PLAY), Default::default())
            .unwrap();
    assert!(restored.take_haptics().unwrap().is_empty());
    assert!(restored.tick(&[]).is_err()); // unsupported until this host advertises anew
}

#[test]
fn haptic_requests_share_the_total_command_quota() {
    for overflow in [false, true] {
        let suffix = if overflow {
            "try{commands.requestRestart()}catch(_){}"
        } else {
            ""
        };
        let mut host=ScriptHost::with_budget(code(&format!("for(let i=0;i<32;i++){{{PLAY}}}for(let i=0;i<{};i++)commands.requestRestart();{suffix}",incant_runtime::MAX_FRAME_COMMANDS-32)),Duration::from_secs(5)).unwrap();
        host.set_haptic_capabilities(capabilities()).unwrap();
        host.enable_restart_requests().unwrap();
        let result = host.tick(&mut world(), 0.1);
        if overflow {
            assert!(matches!(result, Err(ScriptError::InputLimit)));
        } else {
            result.unwrap();
            assert_eq!(host.take_haptics().unwrap().len(), 32);
            assert_eq!(
                host.take_session_request().unwrap(),
                Some(SessionRequest::Restart)
            );
        }
    }
}
#[test]
fn haptic_device_impact_requires_explicit_current_host_authority() {
    let program = code(
        "commands.playHaptic({kind:'device',generation:9},{kind:'impact',intensity:0.25});commands.stopHaptic({kind:'device',generation:9});",
    );
    let mut unsupported = ScriptHost::new(program.clone()).unwrap();
    assert!(unsupported.tick(&mut world(), 0.1).is_err());
    let mut host = ScriptHost::new(program).unwrap();
    host.set_haptic_capabilities(HapticCapabilities::default().with_device(9).unwrap())
        .unwrap();
    host.tick(&mut world(), 0.1).unwrap();
    let output = host.take_haptics().unwrap();
    assert_eq!(output.len(), 2);
    assert_eq!(
        output.iter().next().unwrap().effect,
        HapticEffect::Impact { intensity: 0.25 }
    );
}
#[test]
fn haptic_pending_receiver_refuses_ordinary_restore_without_discarding_output() {
    let scene = incant_runtime::CookedScene::new(id(99), 60, vec![]).unwrap();
    let mut session = NativeSession::new(scene, code(PLAY), Default::default()).unwrap();
    let saved = session.checkpoint_bytes().unwrap();
    session.set_haptic_capabilities(capabilities()).unwrap();
    session.tick(&[]).unwrap();
    let tick = session.world().unwrap().tick();
    assert!(matches!(
        session.restore_checkpoint_bytes(&saved),
        Err(CheckpointError::Unsupported("pending haptic output"))
    ));
    assert_eq!(session.world().unwrap().tick(), tick);
    assert_eq!(session.take_haptics().unwrap().len(), 1);
    session.restore_checkpoint_bytes(&saved).unwrap();
    assert!(session.host.take_haptics().unwrap().is_empty());
    assert!(session.tick(&[]).is_err());
}

#[test]
fn haptic_and_structural_reentrant_calls_share_one_quota() {
    let count = incant_runtime::MAX_FRAME_COMMANDS - 33;
    let one = "00000000000000000000000001";
    let two = "00000000000000000000000002";
    let mut host = ScriptHost::with_budget(
        code(&format!(
            r#"
        chunks[0].transforms[0] = 90;
        for(let i=0;i<32;i++){{{PLAY}}}
        for(let i=0;i<{count};i++)commands.setVelocity('{one}',[1,0,0]);
        const value=[0,0,0];
        Object.defineProperty(value,0,{{get(){{commands.setVelocity('{two}',[2,0,0]);return 3;}}}});
        try{{commands.setVelocity('{one}',value)}}catch(_){{}}
    "#
        )),
        Duration::from_secs(5),
    )
    .unwrap();
    host.set_haptic_capabilities(capabilities()).unwrap();
    let mut world = world();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::InputLimit)
    ));
    assert_eq!(world.snapshot(), before);
    assert!(matches!(
        host.take_haptics(),
        Err(ScriptError::FailedSession)
    ));
}
