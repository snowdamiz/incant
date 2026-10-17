#![cfg(feature = "persistent-conformance")]
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
fn typed() -> SessionProgram {
    CompiledScript::compile(r#"
      exports.default={
        coroutines:{walk:{resume({chunks},task){
          const count=task.state+1;
          for(const c of chunks)for(let i=0;i<c.ids.length;i++){
            c.transforms[i*10]=c.transforms[i*10]*10+(task.id==='a'?1:2);
            c.transforms[i*10+1]++;
          }
          return count<3?{kind:'wait',delayTicks:2,state:count}:{kind:'done'};
        }}},
        update({clock,coroutines}){
          if(clock.tick===1){coroutines.start('z','walk',0);coroutines.start('a','walk',0);coroutines.start('removed','walk',0);}
          if(clock.tick===2)coroutines.cancel('removed');
        }
      };
    "#).unwrap().into()
}
#[test]
fn actual_guests_match_typescript_sorted_callbacks_waits_cancel_and_completion() {
    let mut baseline = NativeSession::new(scene(), typed(), Default::default()).unwrap();
    let mut native = NativeSession::new(
        scene(),
        program(false, "rust-coroutines"),
        Default::default(),
    )
    .unwrap();
    let mut dev = NativeSession::new(
        scene(),
        program(true, "rust-coroutines"),
        Default::default(),
    )
    .unwrap();
    for tick in 1..=8 {
        for host in [&mut baseline, &mut native, &mut dev] {
            host.tick(&[]).unwrap();
        }
        let expected = baseline.world().unwrap().snapshot();
        assert_eq!(
            native.world().unwrap().snapshot(),
            expected,
            "native tick {tick}"
        );
        assert_eq!(dev.world().unwrap().snapshot(), expected, "dev tick {tick}");
    }
    assert_eq!(
        baseline.world().unwrap().snapshot()[0]
            .transform
            .translation,
        [121212., 6., 0.]
    );
}
#[test]
fn pending_frames_survive_fresh_and_persisted_restore_without_restarting_callbacks() {
    use incant_runtime::{
        CheckpointAllocationBudget, CheckpointPayloadBudget, PlanarCheckpointWorkBudget,
        SpatialCheckpointWorkBudget,
    };
    for development in [false, true] {
        let code = program(development, "rust-coroutines");
        let initial = scene();
        let mut baseline =
            NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
        for _ in 0..3 {
            baseline.tick(&[]).unwrap();
        }
        let ordinary = baseline.checkpoint_bytes().unwrap();
        let mut payload = CheckpointPayloadBudget::new(64 * 1024 * 1024).unwrap();
        let mut planar = PlanarCheckpointWorkBudget::new(100_000_000);
        let mut spatial = SpatialCheckpointWorkBudget::new(100_000_000);
        let checkpoint = baseline
            .checkpoint(&mut payload, &mut planar, &mut spatial)
            .unwrap();
        let persisted = checkpoint
            .persisted_bytes(&mut payload, &mut planar, &mut spatial)
            .unwrap();
        let mut fresh = NativeSession::from_checkpoint_bytes(
            &ordinary,
            &initial,
            code.clone(),
            Default::default(),
        )
        .unwrap();
        let mut restored = NativeSession::new(initial, code, Default::default()).unwrap();
        restored
            .restore_persisted_bytes(
                &persisted,
                &mut payload,
                &mut planar,
                &mut spatial,
                &mut CheckpointAllocationBudget::new(64 * 1024 * 1024, 100_000),
            )
            .unwrap();
        for _ in 4..=8 {
            for host in [&mut baseline, &mut fresh, &mut restored] {
                host.tick(&[]).unwrap();
            }
            let expected = baseline.world().unwrap().snapshot();
            assert_eq!(fresh.world().unwrap().snapshot(), expected);
            assert_eq!(restored.world().unwrap().snapshot(), expected);
            assert_eq!(
                fresh.checkpoint_bytes().unwrap(),
                baseline.checkpoint_bytes().unwrap()
            );
            assert_eq!(
                restored.checkpoint_bytes().unwrap(),
                baseline.checkpoint_bytes().unwrap()
            );
        }
    }
}
#[test]
fn ignored_command_errors_and_callback_failures_commit_no_numeric_or_saved_state() {
    for suffix in [
        "unknown",
        "duplicate",
        "scratch",
        "owner",
        "frame",
        "actions",
        "tasks",
        "wait",
        "callback",
        "trap",
    ] {
        let name = format!("rust-coroutines-{suffix}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let mut a = NativeWorld::new(scene());
        let mut b = NativeWorld::new(scene());
        if matches!(suffix, "wait" | "callback" | "trap") {
            a.step().unwrap();
            b.step().unwrap();
            native.tick(&mut a, 1. / 64.).unwrap();
            dev.tick(&mut b, 1. / 64.).unwrap();
        }
        a.step().unwrap();
        b.step().unwrap();
        let before = a.save_bytes().unwrap();
        assert!(native.tick(&mut a, 1. / 64.).is_err(), "{name}");
        assert!(dev.tick(&mut b, 1. / 64.).is_err(), "{name}");
        assert_eq!(a.save_bytes().unwrap(), before, "{name}");
        assert_eq!(b.save_bytes().unwrap(), before, "{name}");
        assert!(native.persistent_checkpoint(&a).is_err());
        assert!(dev.persistent_checkpoint(&b).is_err());
    }
}
#[test]
fn reload_retains_pending_frame_and_incompatible_catalog_refuses_atomically() {
    let code = embedded::linked("rust-coroutines").unwrap();
    let bytes = guest("rust-coroutines");
    let mut native = StaticModule::new(code).unwrap();
    let mut dev = DevModuleHost::new(&bytes).unwrap();
    let mut a = NativeWorld::new(scene());
    let mut b = NativeWorld::new(scene());
    for _ in 0..3 {
        a.step().unwrap();
        b.step().unwrap();
        native.tick(&mut a, 0.25).unwrap();
        dev.tick(&mut b, 0.25).unwrap();
    }
    let checkpoint = native.persistent_checkpoint(&a).unwrap();
    assert_eq!(dev.persistent_checkpoint(&b).unwrap(), checkpoint);
    assert!(
        native
            .reload(embedded::linked("rust-coroutines-schema").unwrap())
            .is_err()
    );
    assert!(dev.reload(&guest("rust-coroutines-schema")).is_err());
    assert_eq!(native.persistent_checkpoint(&a).unwrap(), checkpoint);
    assert_eq!(dev.persistent_checkpoint(&b).unwrap(), checkpoint);
    native.reload(code).unwrap();
    dev.reload(&bytes).unwrap();
    a.step().unwrap();
    b.step().unwrap();
    native.tick(&mut a, 0.25).unwrap();
    dev.tick(&mut b, 0.25).unwrap();
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.snapshot()[0].transform.translation, [1212., 4., 0.]);
}
