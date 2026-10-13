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
    for queued in [0, 1] {
        for overflow in [false, true] {
            let prefix = if queued == 1 {
                "commands.setVelocity('00000000000000000000000001',[3,0,0]);"
            } else {
                ""
            };
            let suffix = if overflow {
                "try {commands.requestRestart()} catch (_) {}"
            } else {
                ""
            };
            // Exercise the structural quota independently of the default CPU
            // deadline, as the other full-buffer command tests do.
            let mut host = ScriptHost::with_budget(
                code(&format!(
                    "chunks[0].transforms[0]=77;{prefix}for(let i=0;i<{};i++)commands.requestRestart();{suffix}",
                    incant_runtime::MAX_FRAME_COMMANDS - queued
                )),
                Duration::from_secs(5),
            )
            .unwrap();
            host.enable_restart_requests().unwrap();
            let mut world = world();
            let before = world.snapshot();
            let result = host.tick(&mut world, 0.1);
            if overflow {
                assert!(matches!(result, Err(ScriptError::InputLimit)), "{result:?}");
                assert_eq!(world.snapshot(), before);
                assert!(matches!(
                    host.take_session_request(),
                    Err(ScriptError::FailedSession)
                ));
            } else {
                result.unwrap();
                assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 77.);
                if queued == 1 {
                    assert_eq!(
                        world.inspect(id(1)).unwrap().velocity.unwrap().linear[0],
                        3.
                    );
                }
                assert_eq!(
                    host.take_session_request().unwrap(),
                    Some(SessionRequest::Restart)
                );
                assert_eq!(host.take_session_request().unwrap(), None);
            }
        }
    }
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
