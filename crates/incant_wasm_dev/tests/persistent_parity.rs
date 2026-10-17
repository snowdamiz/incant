#![cfg(feature = "persistent-conformance")]
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod audio_support;
#[path = "support/persistent.rs"]
mod fixture;
use fixture::*;
use incant_game_runtime::{HostedOfflineGameSession, PendingGameCheckpoint};
use incant_runtime::{CookedScene, NativeWorld};
use incant_script::{NativeSession, SessionProgram, SessionRequest};
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};

fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
fn program(development: bool, name: &str) -> SessionProgram {
    if development {
        SessionProgram::DevelopmentWasm(guest(name))
    } else {
        embedded::linked(name).unwrap().into()
    }
}
fn simple() -> CookedScene {
    CookedScene::new(
        id(99),
        64,
        vec![incant_runtime::CookedEntity {
            id: id(1),
            parent: None,
            velocity: None,
            transform: Default::default(),
        }],
    )
    .unwrap()
}
#[test]
fn actual_legacy_sdk_guest_keeps_explicit_state_abi_refusal() {
    for development in [false, true] {
        let mut live = NativeSession::new(
            simple(),
            program(development, "rust-session-once"),
            Default::default(),
        )
        .unwrap();
        live.enable_restart_requests().unwrap();
        live.tick(&[]).unwrap();
        assert_eq!(
            live.take_session_request().unwrap(),
            Some(SessionRequest::Restart)
        );
        let failure = live.checkpoint_bytes().unwrap_err().to_string();
        assert!(
            failure.contains(if development {
                "development module state ABI"
            } else {
                "static module state ABI"
            }),
            "{failure}"
        );
    }
}

#[test]
fn actual_sdk_counter_survives_ordinary_load_while_guest_globals_start_fresh() {
    for development in [false, true] {
        let code = program(development, "rust-persistent");
        let initial = simple();
        let mut live =
            NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
        for _ in 0..3 {
            live.tick(&[]).unwrap();
        }
        let saved = live.checkpoint_bytes().unwrap();
        let mut restored =
            NativeSession::from_checkpoint_bytes(&saved, &initial, code, Default::default())
                .unwrap();
        assert_ne!(
            live.world().unwrap().incarnation(),
            restored.world().unwrap().incarnation()
        );
        assert_eq!(restored.world().unwrap().tick(), 3);
        assert!(restored.logs().unwrap().entries().is_empty());
        for step in 4..=7 {
            live.tick(&[]).unwrap();
            restored.tick(&[]).unwrap();
            assert_eq!(
                live.world().unwrap().snapshot(),
                restored.world().unwrap().snapshot()
            );
            assert_eq!(
                restored.world().unwrap().snapshot()[0]
                    .transform
                    .translation[0],
                f64::from(step)
            );
        }
        assert_eq!(restored.logs().unwrap().entries()[0].message, "fresh-guest");
        assert_eq!(live.logs().unwrap().entries()[3].message, "continued-guest");
        assert_eq!(
            live.checkpoint_bytes().unwrap(),
            restored.checkpoint_bytes().unwrap()
        );
    }
}

#[test]
fn actual_sdk_transaction_refusals_hide_state_world_and_checkpoints_on_both_hosts() {
    for development in [false, true] {
        for suffix in ["owner", "callback", "quota"] {
            let mut session = NativeSession::new(
                simple(),
                program(development, &format!("rust-persistent-{suffix}")),
                Default::default(),
            )
            .unwrap();
            assert!(session.tick(&[]).is_err());
            assert!(session.world().is_err());
            assert!(session.logs().is_err());
            assert!(session.checkpoint_bytes().is_err());
        }
        let mut session = NativeSession::new(
            simple(),
            program(development, "rust-persistent-restart"),
            Default::default(),
        )
        .unwrap();
        session.enable_restart_requests().unwrap();
        session.tick(&[]).unwrap();
        assert!(session.checkpoint_bytes().is_err());
        assert_eq!(
            session.take_session_request().unwrap(),
            Some(SessionRequest::Restart)
        );
        assert!(session.checkpoint_bytes().is_ok());
    }
}

#[test]
fn actual_sdk_startup_failure_cannot_publish_or_replace_committed_state() {
    let mut a = NativeWorld::new(simple());
    let mut b = NativeWorld::new(simple());
    let mut compiled = StaticModule::new(embedded::linked("rust-persistent").unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest("rust-persistent")).unwrap();
    a.step().unwrap();
    b.step().unwrap();
    compiled.tick(&mut a, 1. / 64.).unwrap();
    dev.tick(&mut b, 1. / 64.).unwrap();
    let before_a = compiled.persistent_checkpoint(&a).unwrap();
    let before_b = dev.persistent_checkpoint(&b).unwrap();
    let failed = embedded::linked("rust-persistent-start").unwrap();
    assert!(StaticModule::new(failed).is_err());
    assert!(DevModuleHost::new(&guest("rust-persistent-start")).is_err());
    assert!(compiled.reload(failed).is_err());
    assert!(dev.reload(&guest("rust-persistent-start")).is_err());
    assert_eq!(compiled.persistent_checkpoint(&a).unwrap(), before_a);
    assert_eq!(dev.persistent_checkpoint(&b).unwrap(), before_b);
    compiled
        .reload(embedded::linked("rust-persistent").unwrap())
        .unwrap();
    dev.reload(&guest("rust-persistent")).unwrap();
    a.step().unwrap();
    b.step().unwrap();
    compiled.tick(&mut a, 1. / 64.).unwrap();
    dev.tick(&mut b, 1. / 64.).unwrap();
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.snapshot()[0].transform.translation[0], 2.);
    assert_eq!(
        compiled.persistent_checkpoint(&a).unwrap(),
        dev.persistent_checkpoint(&b).unwrap()
    );
    assert!(compiled.persistent_checkpoint(&a).unwrap().watermark() > before_a.watermark());
}

#[test]
fn actual_ordinary_native_and_persisted_routes_restore_ui_audio_state_and_fractional_clock() {
    for development in [false, true] {
        for route in 0..3 {
            let temp = tempfile::tempdir().unwrap();
            let (initial, clips) = audible(temp.path());
            let code = program(development, "rust-persistent");
            let mut live = HostedOfflineGameSession::<Host>::new(
                initial.clone(),
                code.clone(),
                Default::default(),
                clips.clone(),
                44117,
                false,
            )
            .unwrap();
            let mut audible = false;
            for _ in 0..13 {
                audible |= tick(&mut live).iter().any(|x| x.abs() > 0.01);
            }
            assert!(audible);
            let world = live.world().unwrap().snapshot();
            let ui = live.host().unwrap().ui.checkpoint_bytes().unwrap();
            let token = live.host().unwrap().ui.focus().unwrap();
            let frames = live.rendered_frames().unwrap();
            let output_count = live.next_output_frames().unwrap();
            let ordinary = (route == 0).then(|| live.checkpoint_bytes().unwrap());
            let native = (route != 0).then(|| {
                live.checkpoint(&mut payload(), &mut planar(), &mut spatial())
                    .unwrap()
            });
            let persisted = (route == 2).then(|| {
                native
                    .as_ref()
                    .unwrap()
                    .persisted_bytes(&mut payload(), &mut planar(), &mut spatial())
                    .unwrap()
            });
            let load = |reject| {
                if let Some(bytes) = &ordinary {
                    HostedOfflineGameSession::<Host>::from_checkpoint_bytes(
                        bytes,
                        &initial,
                        code.clone(),
                        Default::default(),
                        clips.clone(),
                        44117,
                        reject,
                    )
                } else {
                    let pending = if let Some(bytes) = &persisted {
                        PendingGameCheckpoint::from_persisted_bytes(
                            bytes,
                            &initial,
                            code.clone(),
                            Default::default(),
                            &mut payload(),
                            &mut planar(),
                            &mut spatial(),
                            &mut allocations(),
                        )
                        .unwrap()
                    } else {
                        native
                            .as_ref()
                            .unwrap()
                            .prepare(
                                &mut payload(),
                                &mut planar(),
                                &mut spatial(),
                                &mut allocations(),
                            )
                            .unwrap()
                    };
                    HostedOfflineGameSession::<Host>::from_checkpoint(
                        pending,
                        clips.clone(),
                        reject,
                    )
                }
            };
            // A late candidate host failure cannot change the live world, output
            // phase, native VM state, or the exact accepted UI owner/token.
            assert!(load(true).is_err());
            assert_eq!(live.world().unwrap().snapshot(), world);
            assert_eq!(live.rendered_frames().unwrap(), frames);
            assert_eq!(live.next_output_frames().unwrap(), output_count);
            assert_eq!(live.host().unwrap().ui.checkpoint_bytes().unwrap(), ui);
            assert_eq!(live.host().unwrap().ui.focus().unwrap(), token);
            let mut restored = load(false).unwrap();
            assert_ne!(restored.host().unwrap().ui.focus().unwrap(), token);
            assert_ne!(
                live.world().unwrap().incarnation(),
                restored.world().unwrap().incarnation()
            );
            assert_eq!(restored.rendered_frames().unwrap(), frames);
            assert_eq!(restored.host().unwrap().ui.checkpoint_bytes().unwrap(), ui);
            for step in 14..=30 {
                assert_eq!(
                    live.next_output_frames().unwrap(),
                    restored.next_output_frames().unwrap()
                );
                let a = tick(&mut live);
                let b = tick(&mut restored);
                assert_eq!(a, b);
                assert_eq!(
                    live.world().unwrap().snapshot(),
                    restored.world().unwrap().snapshot()
                );
                assert_eq!(
                    restored.world().unwrap().snapshot()[0]
                        .transform
                        .translation[0],
                    f64::from(step)
                );
                assert_eq!(
                    live.host().unwrap().ui.checkpoint_bytes().unwrap(),
                    restored.host().unwrap().ui.checkpoint_bytes().unwrap()
                );
            }
        }
    }
}

#[test]
fn late_composed_refusal_hides_committed_vm_state_and_does_not_publish_pcm() {
    for development in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let (initial, clips) = audible(temp.path());
        let mut game = HostedOfflineGameSession::<Host>::new(
            initial,
            program(development, "rust-persistent"),
            Default::default(),
            clips,
            44117,
            true,
        )
        .unwrap();
        let mut pcm = vec![9.; game.next_output_frames().unwrap() * 2];
        let error = game.tick(&[], &mut pcm).unwrap_err().to_string();
        assert!(error.contains("late composed consumer refusal"), "{error}");
        assert!(game.is_failed());
        assert!(game.world().is_err());
        assert!(game.checkpoint_bytes().is_err());
        assert!(
            game.checkpoint(&mut payload(), &mut planar(), &mut spatial())
                .is_err()
        );
        assert!(pcm.iter().all(|x| *x == 0.));
    }
}
