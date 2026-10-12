#![cfg(feature = "compiler")]
use incant_runtime::*;
use incant_script::{
    CompiledScript, ScriptBindings, ScriptError, ScriptHost, ScriptInstance, ScriptValue,
};
use incant_types::{Animation, Transform};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

const ONE: &str = "00000000000000000000000001";
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn host(source: &str) -> ScriptHost {
    ScriptHost::with_budget(
        CompiledScript::compile(source).unwrap(),
        Duration::from_secs(2),
    )
    .unwrap()
}
fn plain_world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(99),
            8,
            vec![CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }],
        )
        .unwrap(),
    )
}
fn scene(count: usize) -> CookedScene {
    let definition = Arc::new(NativeAnimation {
        mesh_fingerprint: [4; 32],
        clip_durations: Arc::from([2., 4.]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: false,
        },
    });
    CookedScene::new(
        id(0),
        8,
        (1..=count)
            .map(|n| CookedEntity {
                id: id(n as u128),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(u128::MAX),
            kind: VisualAssetKind::Model,
            fingerprint: [4; 32],
        }],
        (1..=count)
            .map(|n| {
                let mut node = CookedSpatial::empty(id(n as u128));
                node.mesh = Some(NativeMeshRenderer {
                    mesh: id(u128::MAX),
                    cast_shadows: false,
                });
                node.animation = Some(Arc::clone(&definition));
                node
            })
            .collect(),
    )
    .unwrap()
}

#[test]
fn actual_typescript_guest_controls_multiple_authored_instances_and_both_source_dispatches() {
    let scene = scene(3);
    let bindings = ScriptBindings::new(
        Sha256::digest(scene.to_bytes()).into(),
        vec![id(102), id(101)],
        [
            (1, 101, 0, -1., false),
            (2, 101, 1, 2., true),
            (3, 102, 0, 16., true),
        ]
        .into_iter()
        .map(|(entity, source, clip, speed, looping)| ScriptInstance {
            entity: id(entity),
            source: id(source),
            props: BTreeMap::from([
                ("clip".into(), ScriptValue::Number(f64::from(clip))),
                ("speed".into(), ScriptValue::Number(speed)),
                ("looping".into(), ScriptValue::Bool(looping)),
            ]),
        })
        .collect(),
    )
    .unwrap();
    let source = format!(
        "{}\nconst shared=exports.default;exports.default={{sources:[{{id:{:?},behavior:shared}},{{id:{:?},behavior:shared}}]}};",
        include_str!("fixtures/animation.cjs"),
        ulid::Ulid::from_bytes(id(101).0).to_string(),
        ulid::Ulid::from_bytes(id(102).0).to_string()
    );
    let compiled =
        CompiledScript::compile_project("animation-guest.cjs", &source, bindings).unwrap();
    let mut world = NativeWorld::new(scene);
    let mut host = ScriptHost::for_world(compiled, &world).unwrap();
    host.tick(&mut world, 0.125).unwrap();
    for (n, clip, speed, looping) in [(1, 0, -1., false), (2, 1, 2., true), (3, 0, 16., true)] {
        let state = world.animation_state(id(n)).unwrap();
        assert_eq!(
            state.sample,
            AnimationSample {
                clip: Some(clip),
                seconds: 0.
            }
        );
        assert_eq!(state.speed, speed);
        assert_eq!(state.looping, looping);
        assert!(!state.playing && !state.completed);
    }
    world.step().unwrap();
    host.tick(&mut world, 0.125).unwrap();
    for n in 1..=3 {
        assert_eq!(world.animation_state(id(n)).unwrap().sample.seconds, 0.5);
    }
}

#[test]
fn committed_bulk_copies_stay_aligned_owned_and_independent_across_controls_and_steps() {
    let mut host = host(&format!(
        r#"
    let tick=0,old,loan;exports.default={{update({{chunks,animations,commands}}){{
        const i=animations.ids.indexOf('{ONE}');
        if(i<0||animations.ids.length!==2||animations.clips.length!==2||animations.seconds.length!==2||animations.speeds.length!==2||animations.flags.length!==2)throw Error('alignment');
        if(loan&&loan.byteLength!==0)throw Error('retained loan');loan=chunks[0].transforms;
        const expected=[[0,0,1,1],[0,0,1,0],[1,0.5,-2,2],[-1,0,-2,2]][tick];
        if([animations.clips[i],animations.seconds[i],animations.speeds[i],animations.flags[i]].some((n,j)=>n!==expected[j]))throw Error('snapshot '+tick);
        if(tick===0){{commands.controlAnimation('{ONE}','playing',false);old=animations;}}
        if(tick===1){{if(old.seconds[i]!==0||old.flags[i]!==1)throw Error('old mutated');old.clips[i]=44;old.seconds[i]=88;old.speeds[i]=99;old.flags[i]=7;try{{old.ids[i]='bad'}}catch(_){{}}commands.controlAnimation('{ONE}','select',1,0.5);commands.controlAnimation('{ONE}','speed',-2);commands.controlAnimation('{ONE}','looping',true);}}
        if(tick===2)commands.controlAnimation('{ONE}','select',null,0);
        if([animations.clips[i],animations.seconds[i],animations.speeds[i],animations.flags[i]].some((n,j)=>n!==expected[j]))throw Error('queued readback');
        tick++;
    }}}};"#
    ));
    let mut world = NativeWorld::new(scene(2));
    for _ in 0..4 {
        host.tick(&mut world, 0.125).unwrap();
        world.step().unwrap();
    }
    assert_eq!(
        world.animation_state(id(1)).unwrap().sample,
        AnimationSample::REST
    );
    assert_eq!(world.animation_state(id(2)).unwrap().sample.seconds, 0.5);
}

#[test]
fn duration_role_and_late_callback_refusal_restore_animation_and_numeric_batch() {
    for tail in [
        format!("commands.controlAnimation('{ONE}','select',2,0);"),
        format!("commands.controlAnimation('{ONE}','select',null,0.125);"),
        format!("commands.controlAnimation('{ONE}','seek',2.0000000001);"),
        "commands.controlAnimation('00000000000000000000000003','playing',false);".into(),
        format!("commands.despawn('{ONE}');"),
        "throw Error('late');".into(),
        "return 1;".into(),
        "void new Promise(()=>{});".into(),
    ] {
        let mut host = host(&format!(
            r#"exports.default={{update({{chunks,commands,log}}){{chunks[0].transforms[0]=55;commands.controlAnimation('{ONE}','playing',false);commands.controlAnimation('{ONE}','speed',-2);commands.setVelocity('{ONE}',[7,0,0]);log('uncommitted');{tail}}}}};"#
        ));
        let mut world = NativeWorld::new(scene(2));
        let before = world.snapshot();
        let animation = world.animation_state(id(1));
        assert!(host.tick(&mut world, 0.125).is_err(), "{tail}");
        assert_eq!(world.snapshot(), before, "{tail}");
        assert_eq!(world.animation_state(id(1)), animation, "{tail}");
        assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
    }
}

#[test]
fn caught_shared_command_quota_rejects_an_actual_animation_prefix() {
    let mut host = host(&format!(
        r#"exports.default={{update({{chunks,commands}}){{chunks[0].transforms[0]=7;try{{for(let i=0;i<{};i++)commands.controlAnimation('{ONE}','playing',false);}}catch(_){{}}}}}};"#,
        MAX_FRAME_COMMANDS + 1
    ));
    let mut world = NativeWorld::new(scene(1));
    let before = world.snapshot();
    let animation = world.animation_state(id(1));
    assert!(matches!(
        host.tick(&mut world, 0.125),
        Err(ScriptError::InputLimit)
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.animation_state(id(1)), animation);
}

#[test]
fn committed_despawn_and_empty_world_have_no_stale_animation_rows() {
    let mut host = host(
        r#"let turn=0;exports.default={update({animations,commands}){if(animations.ids.length!==2-turn)throw Error('membership');if(turn<2)commands.despawn(animations.ids[0]);turn++;}};"#,
    );
    let mut world = NativeWorld::new(scene(2));
    for _ in 0..3 {
        host.tick(&mut world, 0.125).unwrap();
    }
    assert_eq!(world.entity_count(), 0);
}

#[test]
fn once_completion_is_observable_and_seek_clears_it_at_commit() {
    let mut world = NativeWorld::new(scene(1));
    for _ in 0..16 {
        world.step().unwrap();
    }
    let mut host = host(&format!(
        r#"exports.default={{update({{animations,commands}}){{if(animations.clips[0]!==0||animations.seconds[0]!==2||animations.flags[0]!==4)throw Error('completion');commands.controlAnimation('{ONE}','seek',0);commands.controlAnimation('{ONE}','playing',true);if(animations.flags[0]!==4)throw Error('queued readback');}}}};"#
    ));
    host.tick(&mut world, 0.125).unwrap();
    let state = world.animation_state(id(1)).unwrap();
    assert_eq!(state.sample.seconds, 0.);
    assert!(state.playing && !state.completed);
}

#[test]
fn later_source_invalid_control_rolls_back_earlier_instance_and_shared_numeric_work() {
    let scene = scene(2);
    let bindings = ScriptBindings::new(
        Sha256::digest(scene.to_bytes()).into(),
        vec![id(101), id(102)],
        [1, 2]
            .into_iter()
            .map(|n| ScriptInstance {
                entity: id(n),
                source: id(100 + n),
                props: BTreeMap::new(),
            })
            .collect(),
    )
    .unwrap();
    let source = format!(
        r#"exports.default={{sources:[
        {{id:{:?},behavior:{{update({{animations,instances,chunks,commands,log}}){{
            const id=instances[0].entity;if(animations.ids.indexOf(id)<0)throw Error('source A snapshot');
            chunks[0].transforms[0]=777;commands.controlAnimation(id,'select',1,0.5);commands.controlAnimation(id,'playing',false);log('source A staged');
        }}}}}},
        {{id:{:?},behavior:{{update({{animations,instances,commands}}){{
            const id=instances[0].entity;const row=animations.ids.indexOf(id);if(row<0||animations.seconds[row]!==0)throw Error('source B snapshot');
            commands.controlAnimation(id,'speed',2);try{{commands.controlAnimation(id,'seek',Infinity)}}catch(_){{}}
        }}}}}}
    ]}};"#,
        ulid::Ulid::from_bytes(id(101).0).to_string(),
        ulid::Ulid::from_bytes(id(102).0).to_string()
    );
    let compiled =
        CompiledScript::compile_project("late-animation-refusal.cjs", &source, bindings).unwrap();
    let mut world = NativeWorld::new(scene);
    let mut host = ScriptHost::for_world(compiled, &world).unwrap();
    let before = world.snapshot();
    let a = world.animation_state(id(1));
    let b = world.animation_state(id(2));
    assert!(matches!(
        host.tick(&mut world, 0.125),
        Err(ScriptError::CommandInput)
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.animation_state(id(1)), a);
    assert_eq!(world.animation_state(id(2)), b);
    assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
}

#[test]
fn bulk_animation_snapshots_are_vm_accounted_and_allocation_failure_keeps_world() {
    let mut host = host(
        r#"const saved=[];exports.default={update({animations,chunks,commands}){
        const count=animations.ids.length;if(count!==4096||animations.clips.length!==count||animations.seconds.length!==count||animations.speeds.length!==count||animations.flags.length!==count)throw Error('bulk alignment');
        if(new Set(animations.ids).size!==count)throw Error('duplicate ID');
        saved.push(animations);chunks[0].transforms[0]+=1;commands.controlAnimation(animations.ids[count-1],'speed',1);
    }};"#,
    );
    let mut world = NativeWorld::new(scene(4096));
    let mut failed = false;
    for _ in 0..512 {
        let before = world.snapshot();
        match host.tick(&mut world, 0.125) {
            Ok(()) => {}
            Err(error) => {
                assert!(
                    matches!(
                        error,
                        ScriptError::MemoryAllocation | ScriptError::Javascript { .. }
                    ),
                    "{error:?}"
                );
                assert_eq!(world.snapshot(), before);
                assert!(matches!(
                    host.tick(&mut world, 0.125),
                    Err(ScriptError::FailedSession)
                ));
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "retained snapshots escaped the VM memory limit");
}

#[test]
fn native_session_observes_simulated_playback_and_hides_all_outputs_after_refusal() {
    use incant_script::NativeSession;
    let code=CompiledScript::compile(&format!(r#"exports.default={{update({{animations,clock,commands,log}}){{
        if(clock.tick===1){{if(animations.seconds[0]!==0.125||animations.flags[0]!==1)throw Error('fixed step');commands.controlAnimation('{ONE}','select',1,0.5);commands.controlAnimation('{ONE}','speed',-16);commands.controlAnimation('{ONE}','looping',true);commands.controlAnimation('{ONE}','playing',false);log('committed');}}
        else{{if(animations.clips[0]!==1||animations.seconds[0]!==0.5||animations.speeds[0]!==-16||animations.flags[0]!==2)throw Error('committed session');commands.controlAnimation('{ONE}','seek',0.25);try{{commands.controlAnimation('{ONE}','speed',Infinity)}}catch(_){{}}log('uncommitted');}}
    }}}};"#)).unwrap();
    let mut session = NativeSession::new(scene(1), code, Default::default()).unwrap();
    session.tick(&[]).unwrap();
    assert_eq!(
        session
            .world()
            .unwrap()
            .animation_state(id(1))
            .unwrap()
            .sample,
        AnimationSample {
            clip: Some(1),
            seconds: 0.5
        }
    );
    assert_eq!(session.logs().unwrap().entries().len(), 1);
    assert!(session.tick(&[]).is_err());
    assert!(session.is_failed());
    assert!(session.world().is_err());
    assert!(session.input().is_err());
    assert!(session.logs().is_err());
}

#[test]
fn scenes_without_animation_publish_empty_numeric_snapshots_and_expired_controls_stay_expired() {
    let mut host = host(&format!(
        r#"let old;exports.default={{update({{animations,commands,chunks}}){{
        if(!(animations.clips instanceof Int32Array)||!(animations.seconds instanceof Float64Array)||!(animations.speeds instanceof Float64Array)||!(animations.flags instanceof Uint32Array))throw Error('types');
        for(const key of ['ids','clips','seconds','speeds','flags'])if(animations[key].length!==0)throw Error('empty snapshot');
        if(old){{let refused=false;try{{old('{ONE}','speed',1)}}catch(_){{refused=true}}if(!refused)throw Error('old control reactivated')}}
        old=commands.controlAnimation;chunks[0].transforms[0]+=1;
    }}}};"#
    ));
    let mut world = plain_world();
    for n in 1..=3 {
        host.tick(&mut world, 0.125).unwrap();
        assert_eq!(
            world.inspect(id(1)).unwrap().transform.translation[0],
            f64::from(n)
        );
    }
}

#[test]
fn caught_malformed_primitive_controls_reject_all_numeric_and_structural_work() {
    for args in [
        "'select',0",
        "'select',undefined,0",
        "'select',false,0",
        "'select','0',0",
        "'select',-1,0",
        "'select',0.5,0",
        "'select',256,0",
        "'select',NaN,0",
        "'select',Infinity,0",
        "'select',null,-1",
        "'select',0,NaN",
        "'seek'",
        "'seek',-1",
        "'seek',NaN",
        "'seek',Infinity",
        "'seek','1'",
        "'seek',0,0",
        "'playing',1",
        "'playing',new Boolean(true)",
        "'looping','true'",
        "'looping',true,0",
        "'speed',16.0000000001",
        "'speed',-16.0000000001",
        "'speed',NaN",
        "'speed',Infinity",
        "'speed','1'",
        "'unknown'",
        "'s'.repeat(100000)",
        "'\\ud800'",
        "{toString(){throw Error('coercion')}}",
    ] {
        let mut host = host(&format!(
            r#"exports.default={{update({{chunks,commands,log}}){{
            chunks[0].transforms[0]=55;commands.setVelocity('{ONE}',[7,0,0]);
            log('uncommitted');try{{commands.controlAnimation('{ONE}',{args})}}catch(_){{}}
        }}}};"#
        ));
        let mut world = plain_world();
        let before = world.snapshot();
        assert!(
            matches!(host.tick(&mut world, 0.125), Err(ScriptError::CommandInput)),
            "{args}"
        );
        assert_eq!(world.snapshot(), before, "{args}");
        assert!(matches!(host.logs(), Err(ScriptError::FailedSession)));
        assert!(matches!(
            host.tick(&mut world, 0.125),
            Err(ScriptError::FailedSession)
        ));
    }
}
