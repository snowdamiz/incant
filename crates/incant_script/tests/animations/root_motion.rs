use super::*;
use incant_anim::{
    RootMotionCatalog, RootMotionClip, RootMotionDelta, RootMotionTrack,
    track::{Interpolation, Kind},
};
use incant_types::RootMotionApplication;
fn initial() -> CookedScene {
    let base = scene(2);
    let mut nodes = base.spatial().to_vec();
    let catalog = Arc::new(
        RootMotionCatalog::new(
            [4; 32],
            0,
            [0.; 3],
            [0., 0., 0., 1.],
            RootMotionDelta::IDENTITY,
            [2., 4.]
                .into_iter()
                .map(|d| {
                    RootMotionClip::new(
                        d,
                        Some(
                            RootMotionTrack::new(
                                vec![0., d],
                                vec![[0.; 4], [d, 0., 0., 0.]],
                                Interpolation::Linear,
                                Kind::Vector,
                            )
                            .unwrap(),
                        ),
                        None,
                    )
                    .unwrap()
                })
                .collect(),
        )
        .unwrap(),
    );
    for node in &mut nodes {
        Arc::make_mut(node.animation.as_mut().unwrap())
            .initial
            .looping = true;
        node.root_motion = Some(Arc::new(NativeRootMotion {
            catalog: catalog.clone(),
            application: RootMotionApplication::Transform,
            applying: true,
        }));
    }
    base.clone()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap()
}
fn roots(world: &NativeWorld) -> Vec<RootMotionSnapshot> {
    let mut rows = Vec::new();
    world.snapshot_root_motion(&mut rows).unwrap();
    rows
}
#[test]
fn owned_complete_root_columns_and_controls_preserve_exact_clocks_and_committed_readback() {
    let mut world = NativeWorld::new(initial());
    world.step().unwrap();
    let mut host = host(
        r#"let n=0,old,expired;
    for(const key of ['rootMotion','cycles','states','requested','fingerprints'])Object.defineProperty(Object.prototype,key,{set(){throw Error('prototype')}});
    exports.default={update({animations,commands}){
      const s=animations.rootMotion,id=s.ids[0];
      if(!(s.cycles instanceof BigInt64Array)||!(s.ticks instanceof BigUint64Array)||s.ids.length!==2||s.roots.length!==2||s.policies.length!==2||s.flags.length!==2||s.fingerprints.length!==64||s.requested.length!==14||s.applied.length!==14||s.accumulatedApplied.length!==14||s.accumulatedRequested.length!==14)throw Error('alignment');
      if(s.fingerprints.some(x=>x!==4)||s.states.some(x=>x!==null)||s.outgoingStates.some(x=>x!==null)||s.outgoingClips.some(x=>x!==-1)||s.outgoingCycles.some(x=>x!==0n)||s.policies.some(x=>x!==1)||s.roots.some(x=>x!==0))throw Error('identity');
      if(expired){let refused=false;try{expired(id,'applying',true)}catch{refused=true}if(!refused)throw Error('expired')}
      if(n===0){if(s.cycles[0]!==0n||s.ticks[0]!==1n||s.requested[0]!==.125||s.applied[0]!==.125)throw Error('first');commands.controlRootMotion(id,'applying',false);if(s.flags[0]!==1)throw Error('queued');old=s;expired=commands.controlRootMotion;}
      if(n===1){if(old.flags[0]!==1||s.flags[0]!==0||s.applied[0]!==0||s.requested[0]!==.125)throw Error('disabled');old.requested[0]=777;old.cycles[0]=44n;commands.controlRootMotion(id,'reset');commands.controlRootMotion(id,'applying',true);commands.controlAnimation(id,'speed',-2);}
      if(n===2){if(old.requested[0]!==777||old.cycles[0]!==44n||s.cycles[0]!==0n||s.ticks[0]!==3n||s.requested[0]!==-.25||s.accumulatedRequested[0]!==-.25||s.accumulatedApplied[0]!==-.25)throw Error('reset/reverse')}
      n++;
    }}"#,
    );
    for _ in 0..3 {
        host.tick(&mut world, 0.125).unwrap();
        world.step().unwrap();
    }
    assert!(roots(&world)[0].current.cycle < 0);
}
#[test]
fn caught_invalid_root_controls_and_late_failure_roll_back_every_owner_and_numeric_prefix() {
    for tail in [
        "try{commands.controlRootMotion(id,'applying',1)}catch{}",
        "try{commands.controlRootMotion(id,'reset',true)}catch{}",
        "try{commands.controlRootMotion(id,'other')}catch{}",
        "try{commands.controlRootMotion({toString(){throw Error('coerce')}},'reset')}catch{}",
        "commands.controlRootMotion('00000000000000000000000099','reset')",
        "throw Error('late')",
    ] {
        let mut world = NativeWorld::new(initial());
        world.step().unwrap();
        let saved = world.save_bytes().unwrap();
        let mut host = host(&format!(
            "exports.default={{update({{animations,commands,chunks,log}}){{const id=animations.rootMotion.ids[0];chunks[0].transforms[0]=81;commands.controlRootMotion(id,'applying',false);commands.controlRootMotion(animations.rootMotion.ids[1],'reset');log('prefix');{tail};}}}}"
        ));
        assert!(host.tick(&mut world, 0.125).is_err(), "{tail}");
        assert_eq!(world.save_bytes().unwrap(), saved, "{tail}");
        assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
    }
    let mut world = plain_world();
    let saved = world.save_bytes().unwrap();
    let mut host = host(&format!(
        "exports.default={{update({{commands}}){{commands.controlRootMotion('{ONE}','reset')}}}}"
    ));
    assert!(host.tick(&mut world, 0.125).is_err());
    assert_eq!(world.save_bytes().unwrap(), saved);
}
#[test]
fn root_motion_quickjs_sessions_resume_actual_reverse_paused_fades_with_persistent_data() {
    use incant_script::NativeSession;
    let scene = initial();
    let code=CompiledScript::compile(r#"exports.default={persistence:{version:1},update(t){
      let s=t.state.get()||{count:0};s={count:s.count+1};t.state.set(s);if(s.count!==t.clock.tick)throw Error('saved tick');
      const r=t.animations.rootMotion,id=r.ids[0],c=t.commands;
      if(r.ticks[0]!==BigInt(t.clock.tick))throw Error('root tick');
      if(s.count===2)c.controlAnimation(id,'crossfade',1,3.9,1);
      if(s.count===3)c.controlAnimation(id,'speed',-2);
      if(s.count===4)c.controlAnimation(id,'playing',false);
      if(s.count===7)c.controlAnimation(id,'playing',true);
      if(s.count===12)c.controlRootMotion(id,'applying',false);
      if(s.count===15){c.controlRootMotion(id,'reset');c.controlRootMotion(id,'applying',true)}
    }}"#).unwrap();
    for pause_at in [3, 5] {
        let mut baseline =
            NativeSession::new(scene.clone(), code.clone(), Default::default()).unwrap();
        for _ in 0..pause_at {
            baseline.tick(&[]).unwrap();
        }
        let saved = baseline.checkpoint_bytes().unwrap();
        assert_eq!(
            &baseline.world().unwrap().save_bytes().unwrap()[8..12],
            &11u32.to_le_bytes()
        );
        let mut fresh =
            NativeSession::from_checkpoint_bytes(&saved, &scene, code.clone(), Default::default())
                .unwrap();
        let mut in_place =
            NativeSession::new(scene.clone(), code.clone(), Default::default()).unwrap();
        in_place.tick(&[]).unwrap();
        in_place.restore_checkpoint_bytes(&saved).unwrap();
        for _ in 0..48 {
            baseline.tick(&[]).unwrap();
            fresh.tick(&[]).unwrap();
            in_place.tick(&[]).unwrap();
            let expected = baseline.checkpoint_bytes().unwrap();
            assert_eq!(fresh.checkpoint_bytes().unwrap(), expected);
            assert_eq!(in_place.checkpoint_bytes().unwrap(), expected);
            assert_eq!(
                roots(fresh.world().unwrap()),
                roots(baseline.world().unwrap())
            );
        }
    }
}
