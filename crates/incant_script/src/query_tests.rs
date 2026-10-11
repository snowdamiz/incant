use super::*;
use crate::tests::{check, id};
use incant_runtime::{CookedEntity, CookedPhysics, CookedScene};
use incant_types::{BodyMotion, Collider, ColliderShape, RigidBody, Transform, Velocity};
const QUERY: &str = "{origin:[0,0,0],direction:[2,0,0],max_distance:100,include_sensors:false,memberships:4294967295,filter:4294967295}";
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(99),
            60,
            vec![CookedEntity {
                id: id(1),
                parent: None,
                transform: Transform {
                    translation: [5., 0., 0.],
                    ..Default::default()
                },
                velocity: Some(Velocity {
                    linear: [1., 0., 0.],
                }),
            }],
        )
        .unwrap()
        .with_physics(vec![CookedPhysics {
            entity: id(1),
            body: Some(RigidBody {
                motion: BodyMotion::Kinematic,
                ..Default::default()
            }),
            collider: Collider {
                shape: ColliderShape::Box {
                    half_extents: [0.5; 3],
                },
                ..Default::default()
            },
            angular_velocity: None,
        }])
        .unwrap(),
    )
}
fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(
        CompiledScript::compile(&format!(
            "exports.default={{update({{chunks,commands,physics}}){{const query={QUERY};{body}}}}};"
        ))
        .unwrap(),
        Duration::from_secs(1),
    )
    .unwrap()
}
#[test]
fn native_raycast_reads_committed_state_and_returns_owned_schema_values() {
    let mut world = world();
    let mut host = host(
        "const hit=physics.raycast(query);if(hit.entity_id!==chunks[0].ids[0] || Math.abs(hit.distance-4.5)>1e-6 || hit.point[0]!==4.5 || hit.normal[0]!==-1)throw Error('hit');chunks[0].transforms[0]=50;if(physics.raycast(query).distance!==4.5)throw Error('speculative');hit.point[0]=777;hit.entity_id='bad';if(physics.raycast(query).point[0]!==4.5)throw Error('aliased');",
    );
    host.tick(&mut world, 1. / 60.).unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 50.);
    world.step().unwrap();
    let mut next =
        self::host("if(physics.raycast(query).distance<49.5)throw Error('stale solver');");
    next.tick(&mut world, 1. / 60.).unwrap();
}
#[test]
fn retained_functions_never_reactivate_and_result_prototypes_cannot_run_guest_setters() {
    let mut host = host(
        "if(globalThis.saved){let rejected=false;try{saved(query)}catch{rejected=true}if(!rejected)throw Error('old cap');}globalThis.saved=physics.raycast;Object.defineProperty(Array.prototype,'0',{set(){throw Error('array setter')},configurable:true});Object.defineProperty(Object.prototype,'entity_id',{set(){throw Error('object setter')},configurable:true});const hit=physics.raycast(query);delete Array.prototype[0];delete Object.prototype.entity_id;if(hit.distance!==4.5)throw Error('hit');",
    );
    let mut world = world();
    host.tick(&mut world, 0.1).unwrap();
    host.tick(&mut world, 0.1).unwrap();
    check(
        &host,
        &format!(
            "let rejected=false;try{{saved({QUERY})}}catch{{rejected=true}}if(!rejected)throw Error('escaped');"
        ),
    );
}
#[test]
fn caught_invalid_queries_reject_numeric_and_structural_writes() {
    for bad in [
        "query.direction=[0,0,0]",
        "query.memberships=-1",
        "query.include_sensors=1",
        "query.scene_id='other'",
        "Object.defineProperty(query,'origin',{get(){throw Error('getter')}})",
    ] {
        let mut world = world();
        let before = world.snapshot();
        let mut host = host(&format!(
            "chunks[0].transforms[0]=40;commands.despawn(chunks[0].ids[0]);{bad};try{{physics.raycast(query)}}catch{{}}"
        ));
        assert!(host.tick(&mut world, 0.1).is_err(), "{bad}");
        assert_eq!(world.snapshot(), before, "{bad}");
        assert!(matches!(
            host.tick(&mut world, 0.1),
            Err(ScriptError::FailedSession)
        ));
        world.step().unwrap();
        assert!(world.physics_state(id(1)).is_some());
    }
}
#[test]
fn recursive_getters_consume_the_same_bounded_query_budget_without_borrow_panics() {
    let mut world = world();
    let before = world.snapshot();
    let mut host = host(
        "const origin=query.origin;let once=false;Object.defineProperty(query,'origin',{get(){if(!once){once=true;for(let i=0;i<1024;i++){try{physics.raycast({...query,origin})}catch{}}}return origin}});chunks[0].transforms[0]=40;try{physics.raycast(query)}catch{};",
    );
    assert!(matches!(
        host.tick(&mut world, 0.1),
        Err(ScriptError::InputLimit)
    ));
    assert_eq!(world.snapshot(), before);
}
#[test]
fn valid_nested_getter_and_exclusion_work_and_do_not_damage_next_step() {
    let mut world = world();
    let mut control = self::world();
    let mut host = host(
        "const inner={...query};Object.defineProperty(query,'origin',{get(){if(physics.raycast(inner).distance!==4.5)throw Error('nested');return [0,0,0]}});if(physics.raycast(query).distance!==4.5)throw Error('outer');if(physics.raycast({...inner,exclude_entity:chunks[0].ids[0]})!==null)throw Error('excluded');if(physics.raycast({...inner,filter:0})!==null)throw Error('mask');",
    );
    host.tick(&mut world, 0.1).unwrap();
    world.step().unwrap();
    control.step().unwrap();
    assert_eq!(world.physics_state(id(1)), control.physics_state(id(1)));
}
#[test]
fn error_diagnostics_cannot_reactivate_queries_or_column_views() {
    let mut host = host(
        "globalThis.view=chunks[0].transforms;globalThis.ray=physics.raycast;throw Object.defineProperty(new Error(),'message',{get(){let rejected=false;try{ray(query)}catch{rejected=true}globalThis.revoked=rejected&&view.byteLength===0;return 'failure'}});",
    );
    assert!(host.tick(&mut world(), 0.1).is_err());
    check(
        &host,
        "if(!revoked)throw Error('diagnostic capability still live')",
    );
}

#[test]
fn swallowed_result_allocation_failure_still_rolls_back_the_complete_update() {
    let mut world = world();
    let before = world.snapshot();
    let mut host = host(
        "chunks[0].transforms[0]=40;commands.despawn(chunks[0].ids[0]);try{physics.raycast(query)}catch{};",
    );
    queries::FAIL_HIT_ALLOCATION.with(|flag| flag.set(true));
    let result = host.tick(&mut world, 0.1);
    assert!(!queries::FAIL_HIT_ALLOCATION.with(|flag| flag.replace(false)));
    assert!(matches!(result, Err(ScriptError::MemoryAllocation)));
    assert_eq!(world.snapshot(), before);
    world.step().unwrap();
    assert!(world.physics_state(id(1)).is_some());
}
