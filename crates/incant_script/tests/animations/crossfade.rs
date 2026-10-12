use super::*;

#[test]
fn aligned_transition_copies_are_owned_and_queued_control_readback_is_invisible() {
    let mut host = host(&format!(
        r#"let n=0,old;
    exports.default={{update({{animations:a,commands:c}}){{
      const i=a.ids.indexOf('{ONE}');
      for(const k of ['transitionFlags','sourceClips','sourceSeconds','sourceSpeeds','transitionElapsed','transitionDuration'])if(a[k].length!==2)throw Error('alignment');
      if(n===0){{if(a.transitionFlags[i]!==0||a.transitionDuration[i]!==0)throw Error('inactive');c.controlAnimation('{ONE}','crossfade',1,0.5,1);c.controlAnimation('{ONE}','speed',-2);c.controlAnimation('{ONE}','looping',true);old=a;}}
      if(n===1){{if(a.clips[i]!==1||a.seconds[i]!==0.25||a.speeds[i]!==-2||a.flags[i]!==3||a.transitionFlags[i]!==3||a.sourceClips[i]!==0||a.sourceSeconds[i]!==0.125||a.sourceSpeeds[i]!==1||a.transitionElapsed[i]!==0.125||a.transitionDuration[i]!==1)throw Error('fade');
        if(old.transitionFlags[i]!==0||old.sourceSeconds[i]!==0)throw Error('old copy');
        a.sourceSeconds[i]=99;a.transitionElapsed[i]=99;c.controlAnimation('{ONE}','playing',false);}}
      if(n===2){{if(a.sourceSeconds[i]!==0.125||a.transitionElapsed[i]!==0.125||a.flags[i]!==2||a.transitionFlags[i]!==3)throw Error('paused');c.controlAnimation('{ONE}','playing',true);}}
      if(n===3){{if(a.sourceSeconds[i]!==0.25||a.transitionElapsed[i]!==0.25||a.seconds[i]!==0)throw Error('resume');c.controlAnimation('{ONE}','seek',0.5);if(a.transitionFlags[i]!==3)throw Error('queued cancel');}}
      if(n===4){{for(const k of ['transitionFlags','sourceClips','sourceSeconds','sourceSpeeds','transitionElapsed','transitionDuration'])if(a[k][i]!==0)throw Error('inactive suffix');}}
      n++;
    }}}};"#
    ));
    let mut world = NativeWorld::new(scene(2));
    for _ in 0..5 {
        host.tick(&mut world, 0.125).unwrap();
        world.step().unwrap();
    }
    assert!(world.animation_state(id(1)).unwrap().transition.is_none());
    assert_eq!(world.animation_state(id(2)).unwrap().sample.seconds, 0.625);
}

#[test]
fn caught_bad_crossfade_arguments_and_active_retarget_refuse_mixed_work() {
    for args in [
        "0,0",
        "0,0,1,0",
        "'0',0,1",
        "256,0,1",
        "0.5,0,1",
        "null,1,1",
        "0,-1,1",
        "0,NaN,1",
        "0,0,-1",
        "0,0,60.001",
        "0,0,Infinity",
        "0,0,'1'",
    ] {
        let mut host = host(&format!(
            r#"exports.default={{update({{chunks,commands:c,log}}){{chunks[0].transforms[0]=55;c.setVelocity('{ONE}',[7,0,0]);log('uncommitted');try{{c.controlAnimation('{ONE}','crossfade',{args});}}catch(_){{}}}}}};"#
        ));
        let mut world = NativeWorld::new(scene(1));
        let before = world.save_bytes().unwrap();
        assert!(
            matches!(host.tick(&mut world, 0.125), Err(ScriptError::CommandInput)),
            "{args}"
        );
        assert_eq!(world.save_bytes().unwrap(), before);
        assert!(host.logs().is_err());
    }
    for tail in [
        "c.controlAnimation(id,'crossfade',0,0,1)",
        "throw Error('late')",
    ] {
        let mut host = host(&format!(
            r#"exports.default={{update({{animations:a,chunks,commands:c,log}}){{const id=a.ids[0];chunks[0].transforms[0]=55;c.setVelocity(id,[7,0,0]);c.controlAnimation(id,'crossfade',1,0.5,1);log('uncommitted');{tail};}}}};"#
        ));
        let mut world = NativeWorld::new(scene(1));
        let before = world.save_bytes().unwrap();
        assert!(host.tick(&mut world, 0.125).is_err());
        assert_eq!(world.save_bytes().unwrap(), before);
        assert!(host.logs().is_err());
    }
}

#[test]
fn actual_guest_started_fade_resumes_through_fresh_and_in_place_native_session_checkpoint() {
    use incant_script::NativeSession;
    let code = CompiledScript::compile(&format!(
        r#"exports.default={{persistence:{{version:1}},update(t){{
      const a=t.animations,i=a.ids.indexOf('{ONE}');let count=t.state.get()||0;t.state.set(count+1);
      t.chunks[0].transforms[0]=count+a.sourceSeconds[i]+a.transitionElapsed[i];
      if(t.clock.tick===1)t.commands.controlAnimation('{ONE}','crossfade',1,0.5,1);
      if(t.clock.tick===3)t.commands.controlAnimation('{ONE}','playing',false);
      if(t.clock.tick===4)t.commands.controlAnimation('{ONE}','playing',true);
      if(t.clock.tick===5)t.commands.controlAnimation('{ONE}','speed',-0.5);
      if(t.clock.tick===12)t.commands.controlAnimation('{ONE}','crossfade',null,0,0);
      if(t.clock.tick===13)t.commands.controlAnimation('{ONE}','crossfade',0,0,0.5);
      t.log('crossfade accepted');
    }}}};"#
    ))
    .unwrap();
    let initial = scene(2);
    let mut baseline =
        NativeSession::new(initial.clone(), code.clone(), Default::default()).unwrap();
    for _ in 0..4 {
        baseline.tick(&[]).unwrap();
    }
    let saved = baseline.checkpoint_bytes().unwrap();
    let t = baseline
        .world()
        .unwrap()
        .animation_state(id(1))
        .unwrap()
        .transition
        .unwrap();
    assert_eq!((t.elapsed, t.duration, t.source.seconds), (0.25, 1., 0.375));
    let mut fresh =
        NativeSession::from_checkpoint_bytes(&saved, &initial, code.clone(), Default::default())
            .unwrap();
    let mut restored = NativeSession::new(initial, code, Default::default()).unwrap();
    restored.tick(&[]).unwrap();
    restored.restore_checkpoint_bytes(&saved).unwrap();
    assert_eq!(fresh.checkpoint_bytes().unwrap(), saved);
    assert_eq!(restored.checkpoint_bytes().unwrap(), saved);
    for _ in 0..30 {
        baseline.tick(&[]).unwrap();
        fresh.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        let expected = baseline.checkpoint_bytes().unwrap();
        assert_eq!(fresh.checkpoint_bytes().unwrap(), expected);
        assert_eq!(restored.checkpoint_bytes().unwrap(), expected);
    }
    assert!(
        baseline
            .world()
            .unwrap()
            .animation_state(id(1))
            .unwrap()
            .transition
            .is_none()
    );
}

#[test]
fn actual_strict_typescript_cooked_guest_controls_distinct_source_instances() {
    let initial = scene(2);
    let bindings = ScriptBindings::new(
        Sha256::digest(initial.to_bytes()).into(),
        vec![id(101), id(102)],
        [1, 2]
            .into_iter()
            .map(|n| ScriptInstance {
                entity: id(n),
                source: id(100 + n),
                props: BTreeMap::from([("duration".into(), ScriptValue::Number(n as f64))]),
            })
            .collect(),
    )
    .unwrap();
    let source = format!(
        "{}\nconst shared=exports.default;exports.default={{sources:[{{id:{:?},behavior:shared}},{{id:{:?},behavior:shared}}]}};",
        include_str!("../fixtures/crossfade.cjs"),
        ulid::Ulid::from_bytes(id(101).0).to_string(),
        ulid::Ulid::from_bytes(id(102).0).to_string()
    );
    let compiled = CompiledScript::compile_project("crossfade.cjs", &source, bindings).unwrap();
    let mut world = NativeWorld::new(initial);
    let mut host = ScriptHost::for_world(compiled, &world).unwrap();
    host.tick(&mut world, 0.125).unwrap();
    for n in [1, 2] {
        assert_eq!(
            world
                .animation_state(id(n))
                .unwrap()
                .transition
                .unwrap()
                .duration,
            n as f64
        );
    }
    world.step().unwrap();
    host.tick(&mut world, 0.125).unwrap();
    for n in [1, 2] {
        assert_eq!(
            world.animation_state(id(n)).unwrap().sample,
            AnimationSample::REST
        );
        assert!(world.animation_state(id(n)).unwrap().transition.is_none());
    }
}
