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
      let count=0,order=0;
      exports.default={update({chunks,clock,timers}){
        for(const event of timers.events){
          if(event.scheduled_tick!==clock.tick)throw Error('deadline');
          count++;order=order*31+event.id.charCodeAt(0);
        }
        for(const c of chunks)for(let i=0;i<c.ids.length;i++){
          c.transforms[i*10]=clock.tick;c.transforms[i*10+1]=count;
          c.transforms[i*10+2]=clock.elapsed_seconds;c.transforms[i*10+7]=order+1;
        }
        if(clock.tick===1){timers.set('z',2,2);timers.set('a',10);timers.set('a',2);timers.set('removed',1);timers.cancel('removed');timers.set('far',4294967295);}
        if(clock.tick===3){timers.cancel('z');timers.set('a',1);}
        if(clock.tick===4)timers.set('p',2,2);
      }};
    "#).unwrap().into()
}
#[test]
fn actual_rust_guests_match_typescript_order_deadlines_replacement_and_clock() {
    let mut baseline = NativeSession::new(scene(), typed(), Default::default()).unwrap();
    let mut static_host =
        NativeSession::new(scene(), program(false, "rust-timers"), Default::default()).unwrap();
    let mut dev =
        NativeSession::new(scene(), program(true, "rust-timers"), Default::default()).unwrap();
    for tick in 1..=10 {
        for host in [&mut baseline, &mut static_host, &mut dev] {
            host.tick(&[]).unwrap();
        }
        let expected = baseline.world().unwrap().snapshot();
        assert_eq!(static_host.world().unwrap().snapshot(), expected);
        assert_eq!(dev.world().unwrap().snapshot(), expected);
        assert_eq!(expected[0].transform.translation[0], f64::from(tick));
    }
    assert_eq!(
        baseline.world().unwrap().snapshot()[0]
            .transform
            .translation[1],
        6.
    );
}
#[test]
fn actual_guests_continue_pending_timers_after_fresh_and_persisted_restore() {
    use incant_runtime::{
        CheckpointAllocationBudget, CheckpointPayloadBudget, PlanarCheckpointWorkBudget,
        SpatialCheckpointWorkBudget,
    };
    for development in [false, true] {
        let code = program(development, "rust-timers");
        let initial = scene();
        let mut baseline =
            NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
        for _ in 0..4 {
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
        for _ in 5..=10 {
            for host in [&mut baseline, &mut fresh, &mut restored] {
                host.tick(&[]).unwrap();
            }
            let expected = baseline.world().unwrap().snapshot();
            assert_eq!(fresh.world().unwrap().snapshot(), expected);
            assert_eq!(restored.world().unwrap().snapshot(), expected);
        }
    }
}
#[test]
fn actual_ignored_timer_errors_and_late_refusals_do_not_commit_world_or_state() {
    for suffix in [
        "scratch", "delay", "interval", "id", "actions", "pending", "owner", "callback", "trap",
    ] {
        let name = format!("rust-timers-{suffix}");
        let mut a = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut b = DevModuleHost::new(&guest(&name)).unwrap();
        let mut wa = NativeWorld::new(scene());
        let mut wb = NativeWorld::new(scene());
        wa.step().unwrap();
        wb.step().unwrap();
        let before = wa.save_bytes().unwrap();
        assert!(a.tick(&mut wa, 1. / 64.).is_err(), "{name}");
        assert!(b.tick(&mut wb, 1. / 64.).is_err(), "{name}");
        assert_eq!(wa.save_bytes().unwrap(), before, "{name}");
        assert_eq!(wb.save_bytes().unwrap(), before, "{name}");
        assert!(a.persistent_checkpoint(&wa).is_err());
        assert!(b.persistent_checkpoint(&wb).is_err());
    }
}
#[test]
fn actual_reload_retains_timer_schedule_and_refused_startup_does_not_replace_it() {
    let code = embedded::linked("rust-timers").unwrap();
    let bytes = guest("rust-timers");
    let mut a = StaticModule::new(code).unwrap();
    let mut b = DevModuleHost::new(&bytes).unwrap();
    let mut wa = NativeWorld::new(scene());
    let mut wb = NativeWorld::new(scene());
    for _ in 0..2 {
        wa.step().unwrap();
        wb.step().unwrap();
        a.tick(&mut wa, 0.25).unwrap();
        b.tick(&mut wb, 0.25).unwrap();
    }
    let checkpoint = a.persistent_checkpoint(&wa).unwrap();
    assert_eq!(checkpoint, b.persistent_checkpoint(&wb).unwrap());
    let bad = embedded::linked("timers-start").unwrap();
    assert!(StaticModule::new(bad).is_err());
    assert!(a.reload(bad).is_err());
    assert!(b.reload(&guest("timers-start")).is_err());
    assert_eq!(a.persistent_checkpoint(&wa).unwrap(), checkpoint);
    assert_eq!(b.persistent_checkpoint(&wb).unwrap(), checkpoint);
    a.reload(code).unwrap();
    b.reload(&bytes).unwrap();
    wa.step().unwrap();
    wb.step().unwrap();
    a.tick(&mut wa, 0.25).unwrap();
    b.tick(&mut wb, 0.25).unwrap();
    assert_eq!(wa.snapshot(), wb.snapshot());
    assert_eq!(wa.snapshot()[0].transform.translation, [3., 2., 0.75]);
    assert_eq!(wa.snapshot()[0].transform.scale[0], 3130.);
}
