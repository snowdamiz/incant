use super::*;
use crate::tests::{check, id};
use incant_runtime::{CookedEntity, CookedPhysics, CookedScene, StableId};
use incant_types::{BodyMotion, Collider, ColliderShape, RigidBody, Transform, Velocity};

fn scene() -> CookedScene {
    let specs = [
        (1, [0., -0.5, 0.], [20., 0.5, 20.], false),
        (2, [0., 0.6, 0.], [0.5; 3], false),
        (3, [2.5, 1., 0.], [0.5, 2., 4.], false),
        (4, [1., 1., 0.], [1.5, 2., 4.], true),
    ];
    CookedScene::new(
        id(99),
        60,
        specs
            .iter()
            .map(|&(n, translation, _, _)| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform {
                    translation,
                    ..Default::default()
                },
                velocity: (n == 2).then_some(Velocity { linear: [0.; 3] }),
            })
            .collect(),
    )
    .unwrap()
    .with_physics(
        specs
            .iter()
            .map(|&(n, _, half_extents, sensor)| CookedPhysics {
                entity: id(n),
                body: (n == 2).then_some(RigidBody {
                    motion: BodyMotion::Kinematic,
                    ..Default::default()
                }),
                collider: Collider {
                    shape: ColliderShape::Box { half_extents },
                    sensor,
                    ..Default::default()
                },
                angular_velocity: None,
            })
            .collect(),
    )
    .unwrap()
}
fn text(id: StableId) -> String {
    ulid::Ulid::from_bytes(id.0).to_string()
}
fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(CompiledScript::compile(&format!(
        "exports.default={{update({{chunks,commands,physics}}){{const character='{}',wall='{}',sensor='{}';const query={{entity_id:character,translation:[5,-1,0]}};{body}}}}};", text(id(2)), text(id(3)), text(id(4))
    )).unwrap(), Duration::from_secs(1)).unwrap()
}
fn char_x(world: &mut NativeWorld) -> f64 {
    world.inspect(id(2)).unwrap().transform.translation[0]
}
#[test]
fn character_sweep_uses_committed_bodies_and_applies_through_numeric_columns() {
    let mut world = NativeWorld::new(scene());
    let mut host = host(
        "const m=physics.computeCharacterMotion(query);if(!(m.translation[0]>1 && m.translation[0]<1.5 && m.grounded && m.collisions.includes(wall)))throw Error('sweep');const expected=m.translation.slice();for(const c of chunks){const i=c.ids.indexOf(character);if(i>=0){for(let a=0;a<3;a++)c.transforms[i*10+a]+=m.translation[a];}}m.translation[0]=999;m.collisions.length=0;const again=physics.computeCharacterMotion(query);if(again.translation[0]!==expected[0] || !again.collisions.includes(wall))throw Error('speculative or aliased');",
    );
    host.tick(&mut world, 1. / 60.).unwrap();
    let x = char_x(&mut world);
    assert!(x > 1. && x < 1.5);
    world.step().unwrap();
    assert!((world.physics_state(id(2)).unwrap().translation[0] - x).abs() < 1e-5);
}
#[test]
fn trigger_events_are_owned_sorted_nondraining_and_follow_solver_steps() {
    let mut world = NativeWorld::new(scene());
    let mut empty =
        host("if(physics.triggerEvents().length)throw Error('events before first step');");
    empty.tick(&mut world, 1. / 60.).unwrap();
    world.step().unwrap();
    let native = world.physics_events().to_vec();
    assert!(
        native
            .iter()
            .any(|e| e.first == id(2) && e.second == id(4) && e.entered)
    );
    let mut entered = host(
        "const events=physics.triggerEvents();const e=events.find(e=>e.first===character&&e.second===sensor);if(!e || !e.entered)throw Error('enter');e.entered=false;events.length=0;if(!physics.triggerEvents().find(e=>e.first===character&&e.second===sensor)?.entered)throw Error('drained');",
    );
    entered.tick(&mut world, 1. / 60.).unwrap();
    assert_eq!(world.physics_events(), native);
    world.step().unwrap();
    empty.tick(&mut world, 1. / 60.).unwrap();
    let mut move_away = host(
        "for(const c of chunks){const i=c.ids.indexOf(character);if(i>=0)c.transforms[i*10]=10;}if(physics.triggerEvents().length)throw Error('speculative events');",
    );
    move_away.tick(&mut world, 1. / 60.).unwrap();
    world.step().unwrap();
    let mut exited = host(
        "const e=physics.triggerEvents().find(e=>e.first===character&&e.second===sensor);if(!e || e.entered)throw Error('exit');",
    );
    exited.tick(&mut world, 1. / 60.).unwrap();
}
#[test]
fn retained_character_and_event_functions_expire_and_results_bypass_guest_setters() {
    let mut world = NativeWorld::new(scene());
    world.step().unwrap();
    let mut host = host(
        "if(globalThis.previous){for(const f of previous){let rejected=false;try{f()}catch{rejected=true}if(!rejected)throw Error('retained');}}globalThis.previous=[()=>physics.computeCharacterMotion(query),()=>physics.triggerEvents()];Object.defineProperty(Array.prototype,'0',{set(){throw Error('setter')},configurable:true});Object.defineProperty(Object.prototype,'grounded',{set(){throw Error('setter')},configurable:true});const m=physics.computeCharacterMotion(query),events=physics.triggerEvents();delete Array.prototype[0];delete Object.prototype.grounded;if(!m.grounded || !events.length)throw Error('result');",
    );
    host.tick(&mut world, 1. / 60.).unwrap();
    host.tick(&mut world, 1. / 60.).unwrap();
    check(
        &host,
        "for(const f of previous){let rejected=false;try{f()}catch{rejected=true}if(!rejected)throw Error('retained outside');}",
    );
}
#[test]
fn caught_character_or_trigger_errors_reject_all_writes_and_commands() {
    for expression in [
        "physics.computeCharacterMotion({...query,entity_id:wall})",
        "physics.computeCharacterMotion({...query,translation:[NaN,0,0]})",
        "physics.computeCharacterMotion({...query,options:null})",
        "physics.computeCharacterMotion({...query,options:{offset:-1,slide:true,max_slope_climb_angle:0,min_slope_slide_angle:0}})",
        "physics.computeCharacterMotion({...query,get options(){throw Error('getter')}})",
        "physics.computeCharacterMotion({...query,scene_id:'other'})",
        "physics.computeCharacterMotion(query,query)",
        "physics.triggerEvents(1)",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut host = host(&format!(
            "chunks[0].transforms[0]=40;commands.despawn(character);try{{{expression}}}catch{{}}"
        ));
        assert!(host.tick(&mut world, 1. / 60.).is_err(), "{expression}");
        assert_eq!(world.snapshot(), before, "{expression}");
        assert!(matches!(
            host.tick(&mut world, 1. / 60.),
            Err(ScriptError::FailedSession)
        ));
        world.step().unwrap();
        assert!(world.physics_state(id(2)).is_some());
    }
}
#[test]
fn shared_query_budget_and_character_limit_reject_caught_overruns() {
    for body in [
        "for(let i=0;i<129;i++){try{physics.computeCharacterMotion(query)}catch{}}",
        "for(let i=0;i<1024;i++)physics.triggerEvents();try{physics.computeCharacterMotion(query)}catch{}",
        "let once=false;Object.defineProperty(query,'options',{get(){if(!once){once=true;for(let i=0;i<128;i++){try{physics.computeCharacterMotion(query)}catch{}}}return undefined}});try{physics.computeCharacterMotion(query)}catch{}",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut host = host(&format!("chunks[0].transforms[0]=40;{body}"));
        assert!(
            matches!(
                host.tick(&mut world, 1. / 60.),
                Err(ScriptError::InputLimit)
            ),
            "{body}"
        );
        assert_eq!(world.snapshot(), before);
    }
}
#[test]
fn caught_result_allocation_failures_reject_character_and_event_updates() {
    for expression in [
        "physics.computeCharacterMotion(query)",
        "physics.triggerEvents()",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut host = host(&format!(
            "chunks[0].transforms[0]=40;commands.despawn(character);try{{{expression}}}catch{{}}"
        ));
        queries::FAIL_RESULT_ALLOCATION.with(|flag| flag.set(true));
        assert!(matches!(
            host.tick(&mut world, 1. / 60.),
            Err(ScriptError::MemoryAllocation)
        ));
        assert_eq!(world.snapshot(), before);
        world.step().unwrap();
        assert!(world.physics_state(id(2)).is_some());
    }
}
