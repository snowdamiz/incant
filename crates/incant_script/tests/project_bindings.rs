#![cfg(feature = "compiler")]
#[path = "project_bindings/cache.rs"]
mod cache;
use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand,
};
use incant_script::{
    CompiledScript, NativeSession, ScriptBindings, ScriptError, ScriptHost, ScriptInstance,
    ScriptValue,
};
use incant_types::{Transform, Velocity};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn text(n: u128) -> String {
    ulid::Ulid::from(n).to_string()
}
fn entity(n: u128) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform::default(),
        velocity: None,
    }
}
fn scene() -> CookedScene {
    CookedScene::new(id(99), 60, vec![entity(1), entity(2), entity(3)]).unwrap()
}
fn bindings(scene: &CookedScene) -> ScriptBindings {
    ScriptBindings::new(
        Sha256::digest(scene.to_bytes()).into(),
        vec![id(102), id(101)],
        vec![
            ScriptInstance {
                entity: id(3),
                source: id(102),
                props: BTreeMap::from([("speed".into(), ScriptValue::Number(30.))]),
            },
            ScriptInstance {
                entity: id(2),
                source: id(101),
                props: BTreeMap::from([("speed".into(), ScriptValue::Number(20.))]),
            },
            ScriptInstance {
                entity: id(1),
                source: id(101),
                props: BTreeMap::from([("speed".into(), ScriptValue::Number(10.))]),
            },
        ],
    )
    .unwrap()
}
fn compile(before: &str, a: &str, b: &str, bindings: ScriptBindings) -> CompiledScript {
    CompiledScript::compile_project("project-test.cjs", &format!(
        "{before}\nexports.default = {{ sources: [{{ id: {:?}, behavior: {{update(ctx) {{ {a} }} }} }}, {{id: {:?}, behavior: {{update(ctx) {{ {b} }} }} }}] }};", text(101), text(102)), bindings).unwrap()
}
fn start(before: &str, a: &str, b: &str) -> (ScriptHost, NativeWorld) {
    let scene = scene();
    let code = compile(before, a, b, bindings(&scene));
    let world = NativeWorld::new(scene);
    (ScriptHost::for_world(code, &world).unwrap(), world)
}

#[test]
fn sources_run_once_in_id_order_with_immutable_initial_props_and_one_shared_loan() {
    let (mut host, mut world) = start(
        "let calls = 0; let view;",
        r#"
        const {instances, chunks, clock} = ctx;
        if (++calls !== 2 * clock.tick - 1 || instances.length !== 2) throw Error('group A order');
        if (instances[0].props.speed !== 10 || instances[1].props.speed !== 20) throw Error('entity order');
        if (!Object.isFrozen(instances) || !Object.isFrozen(instances[0]) || !Object.isFrozen(instances[0].props)) throw Error('mutable data');
        view = chunks;
        for (const chunk of chunks) for (let row = 0; row < chunk.ids.length; row++) {
            const instance = instances.find(i => i.entity === chunk.ids[row]);
            if (instance) chunk.transforms[row * 10] += instance.props.speed;
        }
    "#,
        r#"
        if (++calls !== 2 * ctx.clock.tick || ctx.instances.length !== 1 || ctx.chunks !== view) throw Error('group B order/loan');
        if (ctx.instances[0].props.speed !== 30) throw Error('wrong props');
        for (const chunk of ctx.chunks) for (let row = 0; row < chunk.ids.length; row++) {
            if (chunk.ids[row] === ctx.instances[0].entity) chunk.transforms[row * 10] += 30;
        }
    "#,
    );
    for _ in 0..3 {
        host.tick(&mut world, 0.25).unwrap();
    }
    for (n, value) in [(1, 30.), (2, 60.), (3, 90.)] {
        assert_eq!(
            world.inspect(id(n)).unwrap().transform.translation[0],
            value
        );
    }
}

#[test]
fn timer_names_and_retained_capabilities_are_scoped_but_budgets_are_global() {
    let (mut host, mut world) = start(
        "let timerA; let oldA;",
        r#"
        const {timers, clock} = ctx;
        if (oldA) { let rejected = false; try { oldA.cancel('same'); } catch (_) { rejected = true; } if (!rejected) throw Error('reactivated prior tick'); }
        if (clock.tick === 1) timers.set('same', 1);
        if (timers.events.length !== (clock.tick === 2 ? 1 : 0)) throw Error('A timing');
        timerA = timers; oldA = timers;
    "#,
        r#"
        const {timers, clock} = ctx;
        let rejected = false; try { timerA.cancel('same'); } catch (_) { rejected = true; }
        if (!rejected) throw Error('reactivated source A');
        if (clock.tick === 1) timers.set('same', 2);
        if (timers.events.length !== (clock.tick === 3 ? 1 : 0)) throw Error('B timing');
        if (timers.events.length && timers.events[0].id !== 'same') throw Error('changed local ID');
    "#,
    );
    for _ in 0..4 {
        host.tick(&mut world, 0.1).unwrap();
    }
    for (a, b) in [
        (
            "for(let i=0;i<64;i++)ctx.timers.set('a'+i,9);",
            "for(let i=0;i<65;i++)ctx.timers.set('b'+i,9);",
        ),
        (
            "for(let i=0;i<128;i++)ctx.timers.cancel('absent');",
            "for(let i=0;i<129;i++){try{ctx.timers.cancel('absent')}catch(_){}}",
        ),
    ] {
        let (mut host, mut world) = start("", a, b);
        assert!(matches!(
            host.tick(&mut world, 0.1),
            Err(ScriptError::InputLimit)
        ));
    }
}

#[test]
fn later_source_failure_rejects_earlier_numeric_structural_log_and_timer_work() {
    for failure in [
        "throw Error('later')",
        "void new Promise(()=>{})",
        "return 1",
        "for(;;){}",
        "try{ctx.timers.set('',1)}catch(_){}",
        "ctx.commands.despawn(ctx.instances[0].entity);ctx.commands.despawn(ctx.instances[0].entity)",
    ] {
        let (mut host, mut world) = start(
            "",
            r#"
            ctx.chunks[0].transforms[0] = 999;
            ctx.timers.set('later',1); ctx.log('uncommitted');
            ctx.commands.setVelocity(ctx.instances[0].entity, [1,2,3]);
        "#,
            failure,
        );
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.1).is_err(), "{failure}");
        assert_eq!(world.snapshot(), before, "{failure}");
        assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
        assert!(matches!(
            host.tick(&mut world, 0.1),
            Err(ScriptError::FailedSession)
        ));
    }
}

#[test]
fn accepted_despawn_prunes_attachment_and_cancels_empty_source_timers() {
    let (mut host, mut world) = start(
        "let removed;",
        r#"
        if (ctx.clock.tick !== 1) throw Error('empty source invoked');
        removed = ctx.instances.map(i=>i.entity);
        for(const instance of ctx.instances)ctx.commands.despawn(instance.entity);
        for(let i=0;i<128;i++)ctx.timers.set('a'+i,1,1);
        if(ctx.instances.length!==2)throw Error('changed membership during update');
    "#,
        r#"
        if(ctx.clock.tick===1) {
            if(ctx.chunks.reduce((n,c)=>n+c.ids.length,0)!==3)throw Error('early structural commit');
        } else if(ctx.clock.tick===2) {
            for(let i=0;i<128;i++)ctx.timers.set('b'+i,2);
            for(const id of removed)ctx.commands.spawn(id,[0,0,0,0,0,0,1,1,1,1],null,null);
        }
    "#,
    );
    for _ in 0..4 {
        host.tick(&mut world, 0.1).unwrap();
    }
    assert_eq!(world.snapshot().len(), 3);
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 0.);
}

#[test]
fn native_entity_lifetime_survives_reparent_and_archetype_move_but_not_id_reuse() {
    let (mut host, mut world) = start("", "ctx.log(String(ctx.instances.length));", "");
    let original = world.entity_identity(id(1));
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetParent {
            entity: id(1),
            parent: Some(id(3)),
        })
        .unwrap();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(1),
            velocity: Some(Velocity {
                linear: [0., 0., 0.],
            }),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    assert_eq!(world.entity_identity(id(1)), original);
    host.tick(&mut world, 0.1).unwrap();
    commands.push(StructuralCommand::Despawn(id(1))).unwrap();
    world.apply(&mut commands).unwrap();
    commands.push(StructuralCommand::Spawn(entity(1))).unwrap();
    world.apply(&mut commands).unwrap();
    assert_ne!(world.entity_identity(id(1)), original);
    host.tick(&mut world, 0.1).unwrap();
    let logs = host.logs().unwrap();
    assert_eq!(
        logs.entries()
            .iter()
            .map(|log| log.message.as_str())
            .collect::<Vec<_>>(),
        vec!["2", "1"]
    );
}

#[test]
fn malformed_or_mismatched_bindings_refuse_before_guest_initialization() {
    let initial = scene();
    let good = bindings(&initial);
    let code = compile(
        "throw Error('should not initialize');",
        "",
        "",
        good.clone(),
    );
    assert!(matches!(
        ScriptHost::new(code.clone()),
        Err(ScriptError::ScriptBindings)
    ));
    let other = NativeWorld::new(CookedScene::new(id(99), 60, vec![entity(1)]).unwrap());
    assert!(matches!(
        ScriptHost::for_world(code, &other),
        Err(ScriptError::ScriptBindings)
    ));
    let mut missing = good.instances().to_vec();
    missing[0].entity = id(999);
    let missing = ScriptBindings::new(
        Sha256::digest(initial.to_bytes()).into(),
        good.sources().to_vec(),
        missing,
    );
    // The pairing uses the exact scene digest; missing entity refs fail at start.
    assert!(missing.is_ok());
    let code = compile(
        "throw Error('should not initialize');",
        "",
        "",
        missing.unwrap(),
    );
    assert!(matches!(
        ScriptHost::for_world(code, &NativeWorld::new(initial)),
        Err(ScriptError::ScriptBindings)
    ));
}

#[test]
fn rejected_input_is_retryable_but_replacing_a_bound_world_is_not() {
    let initial = scene();
    let code = compile(
        "",
        "ctx.log(String(ctx.clock.tick));",
        "",
        bindings(&initial),
    );
    let mut session = NativeSession::new(initial, code, Default::default()).unwrap();
    assert!(
        session
            .tick(&[incant_input::InputEvent::GamepadAxis {
                id: 7,
                axis: incant_input::GamepadAxis::LeftX,
                value: 0.5
            }])
            .is_err()
    );
    session.tick(&[]).unwrap();
    assert_eq!(session.logs().unwrap().entries()[0].message, "1");
    let (mut host, mut world) = start("", "", "");
    let bytes = world.save_bytes().unwrap();
    let previous = world.entity_identity(id(1));
    world.restore_bytes(&bytes).unwrap();
    assert_ne!(world.entity_identity(id(1)), previous);
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::ScriptBindings)
    ));
}

#[test]
fn dangerous_property_names_are_frozen_own_data_even_after_guest_changes_intrinsics() {
    let initial = scene();
    let original = bindings(&initial);
    let mut instances = original.instances().to_vec();
    instances[0].props.insert(
        "__proto__".into(),
        ScriptValue::Object(BTreeMap::from([(
            "constructor".into(),
            ScriptValue::Array(vec![
                ScriptValue::String("data".into()),
                ScriptValue::Bool(true),
                ScriptValue::Null,
                ScriptValue::Number(-0.),
            ]),
        )])),
    );
    let bindings = ScriptBindings::new(
        Sha256::digest(initial.to_bytes()).into(),
        original.sources().to_vec(),
        instances,
    )
    .unwrap();
    let code = compile(
        "Object.freeze = () => {throw Error('guest freeze called')}; Array.isArray=()=>false;",
        r#"
        const props=ctx.instances[0].props, nested=props.__proto__;
        if(Object.getPrototypeOf(props)!==null || Object.getPrototypeOf(nested)!==null)throw Error('prototype');
        if(nested.constructor[0]!=='data' || !Object.is(nested.constructor[3],-0))throw Error('lost own data');
        for(const change of [()=>{props.speed=99},()=>{props.added=1},()=>{nested.constructor[0]='bad'},()=>{nested.constructor.push(1)},()=>{ctx.instances.push(1)}]) {
            let rejected=false; try{change()}catch(_){rejected=true} if(!rejected)throw Error('mutable');
        }
    "#,
        "",
        bindings,
    );
    let mut world = NativeWorld::new(initial);
    let mut host = ScriptHost::for_world(code, &world).unwrap();
    for _ in 0..3 {
        host.tick(&mut world, 0.1).unwrap();
    }
    let mut commands = FrameCommands::default();
    commands.push(StructuralCommand::Despawn(id(2))).unwrap();
    world.apply(&mut commands).unwrap();
    commands.push(StructuralCommand::Spawn(entity(2))).unwrap();
    world.apply(&mut commands).unwrap();
    host.tick(&mut world, 0.1).unwrap();
}

#[test]
fn binding_limits_and_identity_ambiguities_fail_before_vm_allocation() {
    let initial = scene();
    let original = bindings(&initial);
    let hash = Sha256::digest(initial.to_bytes()).into();
    for sources in [
        vec![],
        vec![id(101), id(101), id(102)],
        vec![id(101)],
        vec![id(101), id(102), id(103)],
    ] {
        assert!(ScriptBindings::new(hash, sources, original.instances().to_vec()).is_err());
    }
    let mut duplicate = original.instances().to_vec();
    duplicate[1].entity = duplicate[0].entity;
    assert!(ScriptBindings::new(hash, original.sources().to_vec(), duplicate).is_err());
    let mut deep = ScriptValue::Null;
    for _ in 0..17 {
        deep = ScriptValue::Array(vec![deep]);
    }
    for value in [
        ScriptValue::Number(f64::NAN),
        ScriptValue::Number(f64::INFINITY),
        deep,
        ScriptValue::String("x".repeat(1024 * 1024)),
        ScriptValue::Array(vec![ScriptValue::Null; 65_536]),
    ] {
        let mut instances = original.instances().to_vec();
        instances[0].props.insert("invalid".into(), value);
        assert!(ScriptBindings::new(hash, original.sources().to_vec(), instances).is_err());
    }
    let mut reversed = original.instances().to_vec();
    reversed.reverse();
    assert_eq!(
        ScriptBindings::new(hash, vec![id(102), id(101)], reversed)
            .unwrap()
            .fingerprint(),
        original.fingerprint()
    );
}

#[test]
fn legacy_lifecycle_and_source_catalog_substitution_are_refused() {
    for source in [
        format!(
            "exports.default={{sources:[{{id:{:?},behavior:{{initialState:{{}},update(){{}}}}}},{{id:{:?},behavior:{{update(){{}}}}}}]}};",
            text(101),
            text(102)
        ),
        format!(
            "exports.default={{sources:[{{id:{:?},behavior:{{update(){{}}}}}},{{id:{:?},behavior:{{onTimer(){{}},update(){{}}}}}}]}};",
            text(101),
            text(102)
        ),
        format!(
            "exports.default={{sources:[{{id:{:?},behavior:{{update(){{}}}}}},{{id:{:?},behavior:{{update(){{}}}}}}]}};",
            text(102),
            text(101)
        ),
        "exports.default={update(){}};".into(),
    ] {
        let initial = scene();
        let code =
            CompiledScript::compile_project("invalid-catalog.cjs", &source, bindings(&initial))
                .unwrap();
        assert!(ScriptHost::for_world(code, &NativeWorld::new(initial)).is_err());
    }
}

#[test]
fn authored_updates_cannot_compile_source_through_eval_or_function_constructors() {
    for operation in [
        "eval('1')",
        "(0,eval)('1')",
        "Function('return 1')()",
        "(function(){}).constructor('return 1')()",
        "(async function(){}).constructor('return 1')()",
        "(function*(){}).constructor('yield 1')()",
    ] {
        let (mut host, mut world) = start("", "ctx.chunks[0].transforms[0]=999;", operation);
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.1).is_err(), "{operation}");
        assert_eq!(world.snapshot(), before);
    }
}

#[test]
fn later_source_failure_hides_restart_from_an_earlier_group() {
    let (mut host, mut world) = start(
        "",
        "ctx.commands.requestRestart();ctx.chunks[0].transforms[0]=99;",
        "throw Error('later source failure')",
    );
    host.enable_restart_requests().unwrap();
    assert!(host.tick(&mut world, 0.1).is_err());
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 0.);
    assert!(matches!(
        host.take_session_request(),
        Err(ScriptError::FailedSession)
    ));
}

#[test]
fn coroutine_source_names_owner_retirement_and_checkpoint_pruning_are_native() {
    let scene = scene();
    let code = CompiledScript::compile_project("coroutine-scopes", &format!(r#"
        const make=(increment)=>({{persistence:{{version:1}},coroutines:{{step:{{resume(t,task){{
            t.log(task.id+':'+increment+':'+t.clock.tick);
            for(const chunk of t.chunks)for(let row=0;row<chunk.ids.length;row++)if(chunk.ids[row]===task.entity)chunk.transforms[row*10]+=increment;
            return {{kind:'wait',delayTicks:1,state:task.state}};
        }}}}}},update(t){{
            if(t.clock.tick===1)t.coroutines.start('same','step',null,t.instances[0].entity);
            if(increment===1&&t.clock.tick===2)t.commands.despawn(t.instances[0].entity);
        }}}});
        exports.default={{sources:[{{id:{:?},behavior:make(1)}},{{id:{:?},behavior:make(10)}}]}};
    "#, text(101),text(102)), bindings(&scene)).unwrap();
    let mut baseline = NativeSession::new(scene.clone(), code.clone(), Default::default()).unwrap();
    baseline.tick(&[]).unwrap();
    baseline.tick(&[]).unwrap();
    let saved = baseline.checkpoint_bytes().unwrap();
    let mut restored =
        NativeSession::from_checkpoint_bytes(&saved, &scene, code, Default::default()).unwrap();
    for _ in 0..2 {
        baseline.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
    }
    assert_eq!(
        baseline.world().unwrap().snapshot(),
        restored.world().unwrap().snapshot()
    );
    assert!(baseline.world().unwrap().inspect(id(1)).is_none());
    assert_eq!(
        baseline
            .world()
            .unwrap()
            .inspect(id(3))
            .unwrap()
            .transform
            .translation[0],
        30.
    );
    assert_eq!(
        baseline
            .logs()
            .unwrap()
            .entries()
            .iter()
            .map(|x| x.message.as_str())
            .collect::<Vec<_>>(),
        ["same:1:2", "same:10:2", "same:10:3", "same:10:4"]
    );
    assert_eq!(
        &baseline.logs().unwrap().entries()[2..],
        restored.logs().unwrap().entries()
    );
}

#[test]
fn coroutine_immediate_save_after_last_attachment_deletion_never_restores_a_task() {
    for delete_in_resume in [false, true] {
        for owned in [false, true] {
            let scene = CookedScene::new(id(99), 60, vec![entity(1)]).unwrap();
            let bindings = ScriptBindings::new(
                Sha256::digest(scene.to_bytes()).into(),
                vec![id(101)],
                vec![ScriptInstance {
                    entity: id(1),
                    source: id(101),
                    props: BTreeMap::new(),
                }],
            )
            .unwrap();
            let mut source = r#"
                exports.default={sources:[{id:SOURCE,behavior:{persistence:{version:1},
                    coroutines:{step:{resume(t,task){
                        t.log('resume:'+t.clock.tick);
                        if(RESUME)t.commands.despawn(t.instances[0].entity);
                        return {kind:'wait',delayTicks:1,state:task.state};
                    }}},update(t){
                        if(t.clock.tick===1)t.coroutines.start('task','step',null,OWNER);
                        if(t.clock.tick===2&&!RESUME)t.commands.despawn(t.instances[0].entity);
                    }
                }}]};
            "#
            .replace("SOURCE", &format!("{:?}", text(101)))
            .replace("RESUME", if delete_in_resume { "true" } else { "false" });
            source = source.replace(
                "OWNER",
                if owned {
                    "t.instances[0].entity"
                } else {
                    "undefined"
                },
            );
            let code =
                CompiledScript::compile_project("retired-coroutine", &source, bindings).unwrap();
            let mut baseline =
                NativeSession::new(scene.clone(), code.clone(), Default::default()).unwrap();
            baseline.tick(&[]).unwrap();
            baseline.tick(&[]).unwrap();
            assert!(baseline.world().unwrap().inspect(id(1)).is_none());
            let saved = baseline.checkpoint_bytes().unwrap();
            let mut restored =
                NativeSession::from_checkpoint_bytes(&saved, &scene, code, Default::default())
                    .unwrap();
            assert_eq!(restored.checkpoint_bytes().unwrap(), saved);
            for _ in 0..3 {
                baseline.tick(&[]).unwrap();
                restored.tick(&[]).unwrap();
                assert_eq!(
                    baseline.checkpoint_bytes().unwrap(),
                    restored.checkpoint_bytes().unwrap()
                );
            }
            assert_eq!(baseline.logs().unwrap().entries().len(), 1);
            assert_eq!(baseline.logs().unwrap().entries()[0].message, "resume:2");
            assert!(restored.logs().unwrap().entries().is_empty());
        }
    }
}
