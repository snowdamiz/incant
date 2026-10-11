use super::*;
use crate::tests::{check, id, world};

const ONE: &str = "00000000000000000000000001";
const TWO: &str = "00000000000000000000000002";
const THREE: &str = "00000000000000000000000003";
const IDENTITY: &str = "[0,0,0,0,0,0,1,1,1,1]";

fn code(body: &str) -> CompiledScript {
    CompiledScript::compile(&format!(
        "exports.default = {{update({{chunks, dt, commands}}) {{ {body} }} }};"
    ))
    .unwrap()
}

#[test]
fn structural_changes_follow_numeric_writes_and_reacquire_rows_next_tick() {
    let source = format!(
        r#"
        let turn = 0;
        exports.default = {{update({{chunks, commands}}) {{
            if (turn === 1) {{
                for (const v of saved) if (v.buffer.byteLength !== 0) throw Error('live old view');
                let rejected = false;
                try {{ oldVelocity('{ONE}', [99,0,0]); }} catch (_) {{ rejected = true; }}
                if (!rejected) throw Error('old capability reactivated');
            }}
            for (const c of chunks) for (let row=0; row<c.ids.length; row++) c.transforms[row*10] += 1;
            if (turn === 0) {{
                globalThis.saved = chunks.flatMap(c=>[c.transforms,c.transforms.subarray(1),new DataView(c.transforms.buffer)]);
                globalThis.oldVelocity = commands.setVelocity;
                commands.spawn('{THREE}', new Float64Array([5,0,0,0,0,0,1,1,1,1]), '{ONE}', null);
                commands.setParent('{TWO}', '{THREE}');
                commands.setVelocity('{ONE}', [4,0,0]);
            }} else if (turn === 1) {{
                commands.setParent('{TWO}', null);
                commands.setVelocity('{TWO}', null);
                commands.despawn('{THREE}');
            }}
            turn++;
        }} }};
    "#
    );
    let mut host = ScriptHost::new(CompiledScript::compile(&source).unwrap()).unwrap();
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.entity_count(), 3);
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 7.);
    assert_eq!(
        world.inspect(id(1)).unwrap().velocity.unwrap().linear,
        [4., 0., 0.]
    );
    check(
        &host,
        "let rejected=false; try {oldVelocity('00000000000000000000000001',[90,0,0]);} catch(_){rejected=true;} if(!rejected)throw Error('outside tick');",
    );
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.entity_count(), 2);
    assert_eq!(world.inspect(id(2)).unwrap().parent, None);
    assert_eq!(world.inspect(id(2)).unwrap().velocity, None);
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 2.);
    assert_eq!(
        world.inspect(id(1)).unwrap().velocity.unwrap().linear,
        [4., 0., 0.]
    );
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 3.);
    assert_eq!(world.tick(), 0);
}

#[test]
fn script_growth_relocates_tables_after_every_foreign_view_is_revoked() {
    let mut host = ScriptHost::with_budget(code(&format!(r#"
        if (globalThis.grown) {{
            for (const v of grown) if (v.buffer.byteLength !== 0) throw Error('reattached');
        }} else {{
            globalThis.grown = chunks.flatMap(c=>[c.transforms,c.transforms.subarray(0),new DataView(c.transforms.buffer)]);
            for (let i=1000;i<3048;i++) commands.spawn(String(i).padStart(26,'0'), {IDENTITY}, null, null);
            commands.setVelocity('{ONE}', [2,0,0]);
        }}
        for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10] += 2;
    "#)), Duration::from_secs(2)).unwrap();
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.entity_count(), 2050);
    check(
        &host,
        "for (const v of grown) if(v.buffer.byteLength !== 0) throw Error('attached');",
    );
    host.context.with(|ctx| ctx.run_gc());
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 4.);
    assert_eq!(
        world
            .inspect(incant_runtime::StableId(
                ulid::Ulid::from_string("00000000000000000000001000")
                    .unwrap()
                    .to_bytes()
            ))
            .unwrap()
            .transform
            .translation[0],
        2.
    );
}

#[test]
fn command_arguments_copy_active_views_and_support_later_spawned_parents() {
    let mut host = ScriptHost::new(code(&format!(
        r#"
        const root = chunks.find(c => c.ids.includes('{ONE}'));
        const mover = chunks.find(c => c.ids.includes('{TWO}'));
        root.transforms[0] = 6;
        commands.spawn('{THREE}', root.transforms.subarray(0,10), null, null);
        commands.setVelocity('{ONE}', mover.velocities.subarray(0,3));
        // Subsequent writes must not change the owned structural arguments.
        root.transforms[0] = 9;
        mover.velocities[0] = 8;
        commands.setParent('{THREE}', '00000000000000000000000004');
        commands.spawn('00000000000000000000000004', {IDENTITY}, null, null);
    "#
    )))
    .unwrap();
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    assert_eq!(world.inspect(id(3)).unwrap().transform.translation[0], 6.);
    assert_eq!(world.inspect(id(3)).unwrap().parent, Some(id(4)));
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 9.);
    assert_eq!(
        world.inspect(id(1)).unwrap().velocity.unwrap().linear[0],
        1.
    );
    assert_eq!(
        world.inspect(id(2)).unwrap().velocity.unwrap().linear[0],
        8.
    );
}

#[test]
fn rejected_sync_restores_numeric_prefix_and_entire_structural_batch() {
    for rejected in [
        format!("commands.setParent('{ONE}', '{TWO}');"),
        format!("commands.setParent('{TWO}', '00000000000000000000009999');"),
        format!("commands.despawn('{ONE}');"),
        format!("commands.spawn('{ONE}', {IDENTITY}, null, null);"),
        format!("commands.spawn('{THREE}', [0,0,0,0,0,0,0,1,1,1], null, null);"),
        format!("commands.setVelocity('{TWO}', [NaN,0,0]);"),
        format!("chunks[0].transforms[0]=Infinity; commands.setParent('{TWO}', '{ONE}');"),
    ] {
        let mut host = ScriptHost::new(code(&format!(
            r#"
            globalThis.failedViews = chunks.map(c=>c.transforms);
            globalThis.failedCommand = commands.setVelocity;
            for(const c of chunks) {{c.transforms[0]=23; if(c.velocities)c.velocities[0]=81;}}
            commands.setVelocity('{ONE}', [7,0,0]);
            {rejected}
        "#
        )))
        .unwrap();
        let mut world = world();
        let before = world.snapshot();
        assert!(
            matches!(host.tick(&mut world, 0.1), Err(ScriptError::Structural(_))),
            "{rejected}"
        );
        assert_eq!(world.snapshot(), before, "{rejected}");
        check(
            &host,
            "for(const v of failedViews)if(v.byteLength!==0)throw Error('sync kept loan'); let rejected=false; try{failedCommand('00000000000000000000000001',[9,0,0]);}catch(_){rejected=true;} if(!rejected)throw Error('sync kept capability');",
        );
        assert!(matches!(
            host.tick(&mut world, 0.1),
            Err(ScriptError::FailedSession)
        ));
    }
}

#[test]
fn callback_failure_async_cpu_memory_and_late_returns_discard_commands() {
    for failure in [
        "throw Error('late');",
        "void new Promise(()=>{});",
        "void (async()=>{})();",
        "return 1;",
        "new ArrayBuffer(64*1024*1024);",
        "for(;;){}",
    ] {
        let mut host = ScriptHost::new(code(&format!(
            r#"
            chunks[0].transforms[0]=44;
            commands.spawn('{THREE}', {IDENTITY}, '{ONE}', [4,0,0]);
            commands.setVelocity('{ONE}', [2,0,0]);
            {failure}
        "#
        )))
        .unwrap();
        let mut world = world();
        let before = world.snapshot();
        assert!(host.tick(&mut world, 0.1).is_err(), "{failure}");
        assert_eq!(world.snapshot(), before, "{failure}");
    }
}

#[test]
fn reentrant_argument_getters_hold_no_queue_borrow_and_errors_are_sticky() {
    let mut host = ScriptHost::new(code(&format!(
        r#"
        const v=[0,0,0];
        Object.defineProperty(v,0,{{get(){{commands.setVelocity('{TWO}',[9,0,0]);return 7;}}}});
        commands.setVelocity('{ONE}',v);
    "#
    )))
    .unwrap();
    let mut accepted = world();
    host.tick(&mut accepted, 0.1).unwrap();
    assert_eq!(
        accepted.inspect(id(1)).unwrap().velocity.unwrap().linear,
        [7., 0., 0.]
    );
    assert_eq!(
        accepted.inspect(id(2)).unwrap().velocity.unwrap().linear,
        [9., 0., 0.]
    );

    let mut host = ScriptHost::new(code(&format!(r#"
        chunks[0].transforms[0]=88;
        const v=[0,0,0];
        Object.defineProperty(v,0,{{get(){{commands.setVelocity('{TWO}',[9,0,0]);throw Error('getter');}}}});
        try {{commands.setVelocity('{ONE}',v);}} catch(_) {{}}
    "#))).unwrap();
    let mut rejected = world();
    let before = rejected.snapshot();
    assert!(matches!(
        host.tick(&mut rejected, 0.1),
        Err(ScriptError::CommandInput)
    ));
    assert_eq!(rejected.snapshot(), before);
}

#[test]
fn diagnostics_cannot_reactivate_commands_or_observe_live_views() {
    let mut host = ScriptHost::new(code(&format!(
        r#"
        globalThis.capturedView=chunks[0].transforms;
        capturedView[0]=17;
        const error=new Error();
        Object.defineProperty(error,'message',{{get(){{
            let rejected=false;
            try {{commands.setVelocity('{ONE}',[8,0,0]);}} catch(_) {{rejected=true;}}
            globalThis.diagnosticSafe = rejected && capturedView.byteLength === 0;
            return 'diagnostic';
        }}}});
        throw error;
    "#
    )))
    .unwrap();
    let mut world = world();
    let before = world.snapshot();
    assert!(host.tick(&mut world, 0.1).is_err());
    assert_eq!(world.snapshot(), before);
    check(
        &host,
        "if(!diagnosticSafe)throw Error('unsafe diagnostic');",
    );
}

#[test]
fn strict_structural_inputs_cannot_silently_drop_unsupported_data() {
    for call in [
        "commands.despawn('bad');".to_owned(),
        "commands.despawn('0000000000000000000000000a');".to_owned(),
        format!("commands.spawn('{THREE}', {IDENTITY}, null, null, {{camera:{{}}}});"),
        format!("commands.setParent('{TWO}', undefined);"),
        format!("commands.setVelocity('{ONE}', new Float32Array(3));"),
        format!("commands.setVelocity('{ONE}', [1,2]);"),
        format!("commands.setVelocity('{ONE}', ['1',0,0]);"),
    ] {
        let mut host = ScriptHost::new(code(&format!(
            "chunks[0].transforms[0]=31; try {{{call}}} catch(_) {{}}"
        )))
        .unwrap();
        let mut world = world();
        let before = world.snapshot();
        assert!(
            matches!(host.tick(&mut world, 0.1), Err(ScriptError::CommandInput)),
            "{call}"
        );
        assert_eq!(world.snapshot(), before);
    }
}

#[test]
fn command_limit_is_bounded_and_catching_overflow_cannot_commit_a_prefix() {
    for (count, succeeds) in [
        (incant_runtime::MAX_FRAME_COMMANDS, true),
        (incant_runtime::MAX_FRAME_COMMANDS + 1, false),
    ] {
        let mut host = ScriptHost::with_budget(
            code(&format!(
                r#"
            chunks[0].transforms[0]=19;
            try {{for(let i=0;i<{count};i++) commands.setVelocity('{ONE}',[i,0,0]);}} catch(_) {{}}
        "#
            )),
            Duration::from_secs(5),
        )
        .unwrap();
        let mut world = world();
        let before = world.snapshot();
        let result = host.tick(&mut world, 0.1);
        if succeeds {
            result.unwrap();
            assert_eq!(
                world.inspect(id(1)).unwrap().velocity.unwrap().linear[0],
                (count - 1) as f64
            );
        } else {
            assert!(matches!(result, Err(ScriptError::InputLimit)));
            assert_eq!(world.snapshot(), before);
        }
    }
}

#[test]
fn reentrant_getter_cannot_bypass_the_command_limit() {
    let count = incant_runtime::MAX_FRAME_COMMANDS - 1;
    let mut host = ScriptHost::with_budget(code(&format!(r#"
        chunks[0].transforms[0] = 90;
        for (let i=0; i<{count}; i++) commands.setVelocity('{ONE}', [1,0,0]);
        const v = [0,0,0];
        Object.defineProperty(v, 0, {{get() {{commands.setVelocity('{TWO}', [2,0,0]); return 3;}}}});
        try {{commands.setVelocity('{ONE}', v);}} catch (_) {{}}
    "#)), Duration::from_secs(5)).unwrap();
    let mut world = world();
    let before = world.snapshot();
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::Structural(
            incant_runtime::RuntimeError::BufferFull(_)
        ))
    ));
    assert_eq!(world.snapshot(), before);
}

#[test]
fn physics_domains_and_structural_rejection_restore_all_loans_after_revocation() {
    use incant_runtime::{CookedEntity, CookedPhysics, CookedScene};
    use incant_types::{BodyMotion, Collider, RigidBody, Transform};
    for body in [
        "for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10] = 2000000;",
        "for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10+7] = 2;",
        &format!(
            "for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10] += 1; commands.setParent('{ONE}','{TWO}');"
        ),
        &format!(
            "for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10] += 1; commands.setVelocity('{ONE}',[10001,0,0]);"
        ),
        &format!(
            "for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10] += 1; commands.setVelocity('{ONE}',null);"
        ),
    ] {
        let scene = CookedScene::new(
            id(999),
            60,
            vec![1, 2]
                .into_iter()
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: None,
                    transform: Transform::default(),
                    velocity: None,
                })
                .collect(),
        )
        .unwrap()
        .with_physics(vec![CookedPhysics {
            entity: id(1),
            body: Some(RigidBody {
                motion: BodyMotion::Kinematic,
                ..Default::default()
            }),
            collider: Collider::default(),
            angular_velocity: None,
        }])
        .unwrap();
        let mut world = NativeWorld::new(scene);
        let before = world.snapshot();
        let physics = world.physics_state(id(1));
        let mut host=ScriptHost::new(code(&format!("globalThis.views=chunks.flatMap(c=>[c.transforms,new DataView(c.transforms.buffer)]);{body}"))).unwrap();
        assert!(host.tick(&mut world, 1. / 60.).is_err());
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.physics_state(id(1)), physics);
        check(
            &host,
            "for(const v of views) if(v.buffer.byteLength!==0) throw Error('live rejected physics loan');",
        );
        assert!(matches!(
            host.tick(&mut world, 1. / 60.),
            Err(ScriptError::FailedSession)
        ));
        world.step().unwrap();
        assert_eq!(world.physics_state(id(1)), physics);
    }
}

#[test]
fn planar_domains_and_final_structural_failure_restore_detached_loans() {
    use incant_runtime::{CookedEntity, CookedPhysics2D, CookedScene};
    use incant_types::{BodyMotion, Collider2D, RigidBody, Transform};
    for body in [
        "for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10+7] = 2;".to_owned(),
        "for(const c of chunks) if(c.velocities) c.velocities[2]=1;".to_owned(),
        "for(const c of chunks) for(let i=0;i<c.ids.length;i++) {c.transforms[i*10+3]=0.6; c.transforms[i*10+6]=0.8;}".to_owned(),
        format!("for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10]+=1; commands.setParent('{ONE}','{TWO}');"),
        format!("for(const c of chunks) for(let i=0;i<c.ids.length;i++) c.transforms[i*10]+=1; commands.setVelocity('{ONE}',null);"),
    ] {
        let scene=CookedScene::new(id(999),64,(1..=2).map(|n|CookedEntity{id:id(n),parent:None,transform:Transform::default(),velocity:None}).collect()).unwrap()
            .with_physics_2d(vec![CookedPhysics2D{entity:id(1),body:Some(RigidBody{motion:BodyMotion::Kinematic,..Default::default()}),collider:Collider2D::default(),angular_velocity:None}]).unwrap();
        let mut world=NativeWorld::new(scene);
        let before=world.snapshot();let physics=world.physics_2d_state(id(1));
        let mut host=ScriptHost::new(code(&format!("globalThis.views=chunks.flatMap(c=>[c.transforms,new DataView(c.transforms.buffer)]);{body}"))).unwrap();
        assert!(host.tick(&mut world,1./64.).is_err());
        assert_eq!(world.snapshot(),before);assert_eq!(world.physics_2d_state(id(1)),physics);
        check(&host,"for(const v of views) if(v.buffer.byteLength!==0) throw Error('live rejected planar loan');");
        assert!(matches!(host.tick(&mut world,1./64.),Err(ScriptError::FailedSession)));
        world.step().unwrap();assert_eq!(world.physics_2d_state(id(1)),physics);
    }
}
