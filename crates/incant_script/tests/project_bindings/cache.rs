use super::*;

#[test]
fn frozen_cached_instances_do_not_reactivate_prior_tick_capabilities_or_loans() {
    let (mut host, mut world) = start(
        "let a, b, priorCommands, priorTimers, priorView;",
        r#"
        if(a && a!==ctx.instances)throw Error('unchanged membership rebuilt');
        if(!Object.isFrozen(ctx.instances))throw Error('mutable cached array');
        if(priorCommands){
            for(const f of [()=>priorCommands.setVelocity(ctx.instances[0].entity,[1,2,3]),()=>priorTimers.cancel('old')]){
                let rejected=false;try{f()}catch(_){rejected=true}if(!rejected)throw Error('reactivated capability');
            }
            if(priorView.byteLength!==0)throw Error('old numeric loan still live');
        }
        a=ctx.instances;priorCommands=ctx.commands;priorTimers=ctx.timers;priorView=ctx.chunks[0].transforms;
        for(const change of [()=>ctx.instances.push(1),()=>{ctx.instances[0]=null},()=>{ctx.instances[0].props.speed=900}]){
            let rejected=false;try{change()}catch(_){rejected=true}if(!rejected)throw Error('cached data changed');
        }
        ctx.chunks[0].transforms[0]+=1;
    "#,
        "if(b && b!==ctx.instances)throw Error('other group rebuilt');b=ctx.instances;",
    );
    for _ in 0..4 {
        host.tick(&mut world, 0.1).unwrap();
    }
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 4.);
    assert_eq!(world.inspect(id(1)).unwrap().velocity, None);
}

#[test]
fn old_frozen_snapshot_retains_history_without_reattaching_a_reused_id() {
    let (mut host, mut world) = start(
        "let old, next, other;",
        r#"
        if(!old){old=ctx.instances;return;}
        if(old.length!==2 || old[0].props.speed!==10 || !Object.isFrozen(old))throw Error('old snapshot mutated');
        if(ctx.instances===old || ctx.instances.length!==1 || ctx.instances[0]!==old[1])throw Error('membership/lifetime');
        if(next && next!==ctx.instances)throw Error('replacement snapshot rebuilt');
        next=ctx.instances;ctx.log(ctx.instances[0].entity);
    "#,
        "if(other && other!==ctx.instances)throw Error('unrelated source changed');other=ctx.instances;",
    );
    host.tick(&mut world, 0.1).unwrap();
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(1))).unwrap();
    world.apply(&mut commands).unwrap();
    commands.push(StructuralCommand::Spawn(entity(1))).unwrap();
    world.apply(&mut commands).unwrap();
    for _ in 0..3 {
        host.tick(&mut world, 0.1).unwrap();
    }
    assert_eq!(host.logs().unwrap().entries().len(), 3);
    assert!(
        host.logs()
            .unwrap()
            .entries()
            .iter()
            .all(|e| e.message == text(2))
    );
}

#[test]
fn a_failure_after_a_cache_hit_rolls_back_and_permanently_fails_the_host() {
    let (mut host, mut world) = start(
        "let prior;",
        r#"
        if(prior && prior!==ctx.instances)throw Error('cache miss');prior=ctx.instances;
        ctx.chunks[0].transforms[0]+=1;
        ctx.commands.setVelocity(ctx.instances[0].entity,[ctx.clock.tick,2,3]);
        ctx.log('staged');ctx.timers.set('staged',2);
    "#,
        "if(ctx.clock.tick===2)throw Error('later source rejects cached transaction');",
    );
    host.tick(&mut world, 0.1).unwrap();
    let before = world.snapshot();
    assert!(host.tick(&mut world, 0.1).is_err());
    assert_eq!(world.snapshot(), before);
    assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::FailedSession)
    ));
}

#[test]
fn checkpoint_rebinds_cached_membership_in_a_fresh_vm_and_preserves_explicit_state() {
    let scene = scene();
    let source = format!(
        r#"
        function behavior(){{let prior;return{{persistence:{{version:1}},update(ctx){{
            if(prior && prior!==ctx.instances)throw Error('cache not reused');prior=ctx.instances;
            for(const i of ctx.instances){{
                const n=Number(ctx.state.get(i.entity)||0)+1;ctx.state.set(n,i.entity);
                for(const c of ctx.chunks)for(let row=0;row<c.ids.length;row++)if(c.ids[row]===i.entity)c.transforms[row*10]=n;
            }}
        }}}}}}
        exports.default={{sources:[{{id:{:?},behavior:behavior()}},{{id:{:?},behavior:behavior()}}]}};
        "#,
        text(101),
        text(102)
    );
    let code =
        CompiledScript::compile_project("cache-checkpoint.cjs", &source, bindings(&scene)).unwrap();
    let mut original = NativeSession::new(scene.clone(), code.clone(), Default::default()).unwrap();
    for _ in 0..4 {
        original.tick(&[]).unwrap();
    }
    let mut restored = NativeSession::from_checkpoint_bytes(
        &original.checkpoint_bytes().unwrap(),
        &scene,
        code,
        Default::default(),
    )
    .unwrap();
    for _ in 0..8 {
        original.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        assert_eq!(
            original.checkpoint_bytes().unwrap(),
            restored.checkpoint_bytes().unwrap()
        );
    }
    for n in 1..=3 {
        assert_eq!(
            restored
                .world()
                .unwrap()
                .inspect(id(n))
                .unwrap()
                .transform
                .translation[0],
            12.
        );
    }
}

#[test]
fn a_retained_instance_id_cannot_authorize_state_after_despawn_and_reuse() {
    let scene = scene();
    let source = format!(
        r#"
        let prior;
        exports.default={{sources:[{{id:{:?},behavior:{{persistence:{{version:1}},update(ctx){{
            if(!prior){{prior=ctx.instances;ctx.state.set(7,prior[0].entity);return;}}
            if(ctx.instances.length!==1 || prior.length!==2)throw Error('membership');
            // Catching this refusal must still fail the complete transaction.
            try{{ctx.state.set(99,prior[0].entity)}}catch(_){{}}
            ctx.chunks[0].transforms[0]=888;
        }}}}}},{{id:{:?},behavior:{{persistence:{{version:1}},update(){{}}}}}}]}};
        "#,
        text(101),
        text(102)
    );
    let code =
        CompiledScript::compile_project("cache-retired-authority.cjs", &source, bindings(&scene))
            .unwrap();
    let mut world = NativeWorld::new(scene);
    let mut host = ScriptHost::for_world(code, &world).unwrap();
    host.tick(&mut world, 0.1).unwrap();
    let mut changes = FrameCommands::default();
    changes.push(StructuralCommand::Despawn(id(1))).unwrap();
    world.apply(&mut changes).unwrap();
    changes.push(StructuralCommand::Spawn(entity(1))).unwrap();
    world.apply(&mut changes).unwrap();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::PersistenceInput)
    ));
    assert_eq!(world.snapshot(), before);
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::FailedSession)
    ));
}
