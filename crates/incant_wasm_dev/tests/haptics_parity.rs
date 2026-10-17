#![cfg(feature = "persistent-conformance")]
#[path = "../../incant_assets_runtime/tests/support/mod.rs"]
mod audio_support;
#[path = "haptics/checkpoints.rs"]
mod checkpoints;
use incant_game_runtime::{HostedOfflineGameSession, SceneHost};
use incant_input::haptics::{HapticCapabilities, HapticEffect, HapticRequest, HapticTarget};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_script::{CompiledScript, NativeSession, SessionProgram};
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
fn scene() -> CookedScene {
    CookedScene::new(
        StableId([9; 16]),
        64,
        vec![CookedEntity {
            id: StableId(1u128.to_be_bytes()),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap()
}
fn caps() -> HapticCapabilities {
    HapticCapabilities::default()
        .with_gamepad(0, 7)
        .unwrap()
        .with_device(9)
        .unwrap()
}
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
fn expected() -> Vec<HapticRequest> {
    let gamepad = HapticTarget::Gamepad {
        id: 0,
        generation: 7,
    };
    let device = HapticTarget::Device { generation: 9 };
    vec![
        HapticRequest {
            target: gamepad,
            effect: HapticEffect::Rumble {
                strong: 0.5,
                weak: 1.,
                duration_ms: 125,
            },
        },
        HapticRequest {
            target: gamepad,
            effect: HapticEffect::Stop,
        },
        HapticRequest {
            target: device,
            effect: HapticEffect::Impact { intensity: 0.25 },
        },
        HapticRequest {
            target: device,
            effect: HapticEffect::Stop,
        },
    ]
}
#[test]
fn actual_rust_guest_matches_typed_script_and_pending_output_preserves_world_input() {
    let js=CompiledScript::compile("let n=0;exports.default={update(t){for(const c of t.chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*10]=++n;for(const target of t.commands.hapticCapabilities()){t.commands.playHaptic(target,target.kind==='gamepad'?{kind:'rumble',strong:.5,weak:1,durationMs:125}:{kind:'impact',intensity:.25});t.commands.stopHaptic(target)}}}").unwrap();
    for code in [
        js.into(),
        program(false, "rust-haptics"),
        program(true, "rust-haptics"),
    ] {
        let mut live = NativeSession::new(scene(), code, Default::default()).unwrap();
        assert!(live.supports_haptics().unwrap());
        live.set_haptic_capabilities(caps()).unwrap();
        for turn in 1..=3 {
            live.tick(&[]).unwrap();
            assert_eq!(
                live.world().unwrap().snapshot()[0].transform.translation[0],
                f64::from(turn)
            );
            let before = live.world().unwrap().save_bytes().unwrap();
            let input = live.input().unwrap().clone();
            let error = live
                .tick(&[incant_input::InputEvent::Key {
                    code: incant_input::KeyCode::KeyD,
                    down: true,
                }])
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("previous haptic output has not been consumed"),
                "{error}"
            );
            assert!(!live.is_failed());
            assert_eq!(live.world().unwrap().save_bytes().unwrap(), before);
            assert_eq!(live.input().unwrap(), &input);
            assert!(live.set_haptic_capabilities(Default::default()).is_err());
            assert_eq!(
                live.take_haptics().unwrap().iter().collect::<Vec<_>>(),
                expected()
            );
            assert!(live.take_haptics().unwrap().is_empty());
        }
        live.clear_haptics();
        live.tick(&[]).unwrap();
        assert!(live.take_haptics().unwrap().is_empty());
    }
}
#[test]
fn actual_sdk_ignored_errors_and_late_refusals_discard_numeric_and_haptic_prefixes() {
    for suffix in ["scratch", "amplitude", "quota", "owner", "callback", "trap"] {
        let name = format!("rust-haptics-{suffix}");
        let mut a = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut b = DevModuleHost::new(&guest(&name)).unwrap();
        a.set_haptic_capabilities(caps()).unwrap();
        b.set_haptic_capabilities(caps()).unwrap();
        let mut wa = NativeWorld::new(scene());
        let mut wb = NativeWorld::new(scene());
        wa.step().unwrap();
        wb.step().unwrap();
        let before = wa.save_bytes().unwrap();
        let static_error = a.tick(&mut wa, 1. / 64.).unwrap_err();
        let development_error = b.tick(&mut wb, 1. / 64.).unwrap_err();
        if suffix == "owner" {
            let missing = StableId(999u128.to_be_bytes());
            assert!(matches!(static_error,
                incant_wasm_runtime::StaticError::Boundary(
                    incant_wasm_runtime::BoundaryError::Runtime(
                        incant_runtime::RuntimeError::MissingEntity(id))) if id == missing));
            assert!(matches!(development_error,
                incant_wasm_dev::ModuleError::Runtime(
                    incant_runtime::RuntimeError::MissingEntity(id)) if id == missing));
        }
        assert_eq!(wa.save_bytes().unwrap(), before);
        assert_eq!(wb.save_bytes().unwrap(), before);
        assert!(a.take_haptics().is_err());
        assert!(b.take_haptics().is_err());
        assert!(a.persistent_checkpoint(&wa).is_err());
        assert!(b.persistent_checkpoint(&wb).is_err());
    }
}
#[test]
fn retained_guest_targets_cannot_reuse_disconnected_device_generation() {
    for development in [false, true] {
        let mut live = NativeSession::new(
            scene(),
            program(development, "rust-haptics-saved_target"),
            Default::default(),
        )
        .unwrap();
        live.set_haptic_capabilities(caps()).unwrap();
        live.tick(&[]).unwrap();
        assert_eq!(live.take_haptics().unwrap().len(), 4);
        live.set_haptic_capabilities(HapticCapabilities::default().with_gamepad(0, 8).unwrap())
            .unwrap();
        assert!(live.tick(&[]).is_err());
        assert!(live.is_failed());
        assert!(live.take_haptics().is_err());
    }
}
#[test]
fn failed_reload_and_startup_leave_pending_output_drainable_then_reload_is_neutral() {
    let mut a = StaticModule::new(embedded::linked("rust-haptics").unwrap()).unwrap();
    let mut b = DevModuleHost::new(&guest("rust-haptics")).unwrap();
    let mut wa = NativeWorld::new(scene());
    let mut wb = NativeWorld::new(scene());
    a.set_haptic_capabilities(caps()).unwrap();
    b.set_haptic_capabilities(caps()).unwrap();
    wa.step().unwrap();
    wb.step().unwrap();
    a.tick(&mut wa, 1. / 64.).unwrap();
    b.tick(&mut wb, 1. / 64.).unwrap();
    let bad = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/haptics-start.wat"
    ))
    .unwrap();
    let bad_static = embedded::linked("haptics-start").unwrap();
    assert!(StaticModule::new(bad_static).is_err());
    assert!(DevModuleHost::new(&bad).is_err());
    assert!(a.reload(bad_static).is_err());
    assert!(b.reload(&bad).is_err());
    assert!(a.reload(embedded::linked("rust-haptics").unwrap()).is_err());
    assert!(b.reload(&guest("rust-haptics")).is_err());
    assert_eq!(
        a.take_haptics().unwrap().iter().collect::<Vec<_>>(),
        expected()
    );
    assert_eq!(
        b.take_haptics().unwrap().iter().collect::<Vec<_>>(),
        expected()
    );
    let before = a.persistent_checkpoint(&wa).unwrap();
    assert_eq!(before, b.persistent_checkpoint(&wb).unwrap());
    a.reload(embedded::linked("rust-haptics").unwrap()).unwrap();
    b.reload(&guest("rust-haptics")).unwrap();
    wa.step().unwrap();
    wb.step().unwrap();
    a.tick(&mut wa, 1. / 64.).unwrap();
    b.tick(&mut wb, 1. / 64.).unwrap();
    assert_eq!(wa.snapshot()[0].transform.translation[0], 2.);
    assert_eq!(wa.snapshot(), wb.snapshot());
    assert!(a.take_haptics().unwrap().is_empty());
    assert!(b.take_haptics().unwrap().is_empty());
}
struct Host {
    ticks: u32,
    observed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    reject: bool,
}
impl SceneHost for Host {
    type Settings = (std::sync::Arc<std::sync::atomic::AtomicBool>, bool);
    type Error = std::io::Error;
    fn prepare(_: &NativeWorld, (observed, reject): Self::Settings) -> Result<Self, Self::Error> {
        Ok(Self {
            ticks: 0,
            observed,
            reject,
        })
    }
    fn sync(&mut self, world: &NativeWorld) -> Result<(), Self::Error> {
        self.ticks += 1;
        assert_eq!(
            world.snapshot()[0].transform.translation[0],
            f64::from(self.ticks)
        );
        self.observed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if self.reject {
            Err(std::io::Error::other("late haptic consumer refusal"))
        } else {
            Ok(())
        }
    }
}
#[test]
fn composed_audio_and_consumer_acceptance_gate_actual_haptics_and_pcm() {
    let root = tempfile::tempdir().unwrap();
    let key = audio_support::cooked(root.path(), &vec![[0.25, -0.125]; 4096]);
    let id = StableId(90u128.to_be_bytes());
    let clip = incant_audio_runtime::ClipBinding::load(id, root.path(), &key).unwrap();
    let audible = scene()
        .with_audio(vec![incant_runtime::CookedAudio {
            entity: StableId(1u128.to_be_bytes()),
            bus: None,
            listener: false,
            source: Some(incant_runtime::NativeAudioSource {
                transport_epoch: 0,
                clip: id,
                bus: None,
                gain_db: 0.,
                pan: 0.,
                rate: 1.,
                start_seconds: 0.,
                playing: true,
                looping: true,
                streaming: false,
                spatial: None,
            }),
        }])
        .unwrap();
    for development in [false, true] {
        for reject in [false, true] {
            let observed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let mut live = HostedOfflineGameSession::<Host>::new(
                audible.clone(),
                program(development, "rust-haptics"),
                Default::default(),
                vec![clip.clone()],
                44117,
                (observed.clone(), reject),
            )
            .unwrap();
            live.set_haptic_capabilities(caps()).unwrap();
            let mut pcm = vec![9.; live.next_output_frames().unwrap() * 2];
            let result = live.tick(&[], &mut pcm);
            assert!(observed.load(std::sync::atomic::Ordering::SeqCst));
            if reject {
                let error = result.unwrap_err();
                assert!(
                    error.to_string().contains("late haptic consumer refusal"),
                    "{error}"
                );
                assert!(live.is_failed());
                assert!(live.take_haptics().is_err());
                assert!(pcm.iter().all(|v| *v == 0.));
            } else {
                result.unwrap();
                assert!(pcm.iter().any(|v| v.abs() > 0.01));
                assert_eq!(
                    live.take_haptics().unwrap().iter().collect::<Vec<_>>(),
                    expected()
                );
                // Produce another accepted but undrained request set. Suspension
                // must discard it and clear authority, not merely empty a queue.
                let mut pcm = vec![0.; live.next_output_frames().unwrap() * 2];
                live.tick(&[], &mut pcm).unwrap();
                live.set_suspended(true).unwrap();
                assert!(live.take_haptics().unwrap().is_empty());
                live.set_suspended(false).unwrap();
                let mut pcm = vec![0.; live.next_output_frames().unwrap() * 2];
                live.tick(&[], &mut pcm).unwrap();
                assert_eq!(live.world().unwrap().tick(), 3);
                assert!(live.take_haptics().unwrap().is_empty());
            }
        }
    }
}
