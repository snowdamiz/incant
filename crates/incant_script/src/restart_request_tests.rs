use super::*;
use crate::tests::{check, id, world};

fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default={{update({{chunks,commands,clock}}){{{body}}}}};"
    ))
    .unwrap()
}
#[test]
fn unsupported_and_caught_invalid_restart_requests_reject_numeric_prefixes() {
    for (enabled, body) in [
        (false, "try {commands.requestRestart()} catch (_) {}"),
        (true, "try {commands.requestRestart(1)} catch (_) {}"),
        (
            true,
            "commands.requestRestart(); throw Error('late callback')",
        ),
        (
            true,
            "commands.requestRestart(); commands.despawn('00000000000000000000000999')",
        ),
    ] {
        let mut host =
            ScriptHost::new(code(&format!("chunks[0].transforms[0]=77;{body}"))).unwrap();
        if enabled {
            host.enable_restart_requests().unwrap();
        }
        let mut world = world();
        let before = world.inspect(id(1)).unwrap().transform;
        assert!(host.tick(&mut world, 0.1).is_err());
        assert_eq!(world.inspect(id(1)).unwrap().transform, before);
        assert!(matches!(
            host.take_session_request(),
            Err(ScriptError::FailedSession)
        ));
    }
}
#[test]
fn accepted_restart_coalesces_and_expired_function_cannot_request_again() {
    let mut host = ScriptHost::new(code("if(clock.tick===1){globalThis.old=commands.requestRestart;commands.requestRestart();commands.requestRestart();}else{let refused=false;try{old()}catch(_){refused=true}if(!refused)throw Error('revoked')}chunks[0].transforms[0]+=1;" )).unwrap();
    host.enable_restart_requests().unwrap();
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(
        host.take_session_request().unwrap(),
        Some(SessionRequest::Restart)
    );
    assert_eq!(host.take_session_request().unwrap(), None);
    check(
        &host,
        "let refused=false;try{old()}catch(_){refused=true}if(!refused)throw Error('outside tick')",
    );
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(host.take_session_request().unwrap(), None);
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 2.);
}
#[test]
fn restart_calls_share_structural_quota_including_caught_overflow() {
    let mut host=ScriptHost::new(code(&format!("for(let i=0;i<{};i++)commands.requestRestart();try{{commands.requestRestart()}}catch(_){{}}",incant_runtime::MAX_FRAME_COMMANDS))).unwrap();
    host.enable_restart_requests().unwrap();
    assert!(matches!(
        host.tick(&mut world(), 0.1),
        Err(ScriptError::InputLimit)
    ));
    assert!(host.take_session_request().is_err());
}

#[test]
fn pending_restart_cannot_disappear_through_a_checkpoint() {
    let scene = incant_runtime::CookedScene::new(id(99), 60, vec![]).unwrap();
    let program = CompiledScript::compile(
        "exports.default={persistence:{version:1},update(t){t.commands.requestRestart()}};",
    )
    .unwrap();
    let mut session = NativeSession::new(scene, program, Default::default()).unwrap();
    session.enable_restart_requests().unwrap();
    session.tick(&[]).unwrap();
    assert!(matches!(
        session.checkpoint_bytes(),
        Err(CheckpointError::Unsupported(
            "pending session restart request"
        ))
    ));
    assert_eq!(
        session.take_session_request().unwrap(),
        Some(SessionRequest::Restart)
    );
    assert!(session.checkpoint_bytes().is_ok());
}
