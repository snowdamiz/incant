//! Actual VM/native conversions, not a synthetic replacement of the query path.
use super::*;
use crate::{localization_tests::fixture as localization, navigation_tests::fixture as navigation};
use incant_profile::diagnostic::{Report, queries::Capability};
use incant_runtime::{CookedEntity, CookedPhysics, CookedScene, NativeLocalization, StableId};
use incant_types::{BodyMotion, Collider, ColliderShape, RigidBody, Transform, Velocity};

fn scene() -> CookedScene {
    CookedScene::new(
        StableId([99; 16]),
        60,
        vec![CookedEntity {
            id: StableId([1; 16]),
            parent: None,
            transform: Transform::default(),
            velocity: Some(Velocity { linear: [0.; 3] }),
        }],
    )
    .unwrap()
    .with_physics(vec![CookedPhysics {
        entity: StableId([1; 16]),
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
    .unwrap()
    .with_localization(
        NativeLocalization::new(localization::tables(), localization::settings("en")).unwrap(),
    )
    .unwrap()
}
fn host(body: &str) -> ScriptHost {
    let id = ulid::Ulid::from_bytes([1; 16]).to_string();
    ScriptHost::with_budget(CompiledScript::compile(&format!(
        "exports.default={{update({{chunks,commands,physics,localization,navigation}}){{const entity='{id}',table='{}';{body}}}}};",localization::TABLE
    )).unwrap(), Duration::from_secs(3)).unwrap()
}
fn recorded(script: &mut ScriptHost, world: &mut NativeWorld) -> (Result<(), ScriptError>, Report) {
    let capture = incant_profile::Capture::start().unwrap();
    let result = incant_profile::frame(|| script.tick(world, 1. / 60.));
    (result, capture.report().script_phase_diagnostic)
}
// These scenes own physics; legacy save_bytes deliberately refuses them. Check
// the actual public component/solver observations, then advance both worlds to
// expose a leaked queued command or damaged retained physics owner.
fn assert_worlds_equal(observed: &NativeWorld, control: &NativeWorld) {
    assert_eq!(observed.tick(), control.tick());
    assert_eq!(observed.snapshot(), control.snapshot());
    assert_eq!(observed.physics_events(), control.physics_events());
    assert_eq!(
        observed.localization().settings(),
        control.localization().settings()
    );
    for entity in observed.snapshot() {
        assert_eq!(
            observed.physics_state(entity.id),
            control.physics_state(entity.id)
        );
        assert_eq!(
            observed.physics_configuration(entity.id),
            control.physics_configuration(entity.id)
        );
    }
}
fn assert_same_after_next_step(observed: &mut NativeWorld, control: &mut NativeWorld) {
    assert_worlds_equal(observed, control);
    observed.step().unwrap();
    control.step().unwrap();
    assert_worlds_equal(observed, control);
}
#[test]
fn query_diagnostic_records_real_core_capabilities_without_changing_owned_results() {
    let body = r#"
        const moved=physics.computeCharacterMotion({entity_id:entity,translation:[1,0,0]});
        if(moved.translation[0]!==1||moved.collisions.length!==0)throw Error('movement');
        if(physics.triggerEvents().length!==0||localization.locale().locale!=='en')throw Error('state');
        const text=localization.localize({table_id:table,key:'hello',arguments:{name:'Ada'}});
        if(text.text!=='Hello Ada'||localization.localize({table_id:table,key:'absent'}).missing.kind!=='key')throw Error('text');
        if(localization.formatNumber(2)!=='2')throw Error('number');
        chunks[0].transforms[0]=moved.translation[0];
    "#;
    let mut control = NativeWorld::new(scene());
    host(body).tick(&mut control, 1. / 60.).unwrap(); // same feature, no active recording
    let mut observed = NativeWorld::new(scene());
    let (result, report) = recorded(&mut host(body), &mut observed);
    result.unwrap();
    assert_same_after_next_step(&mut observed, &mut control);
    assert!(report.complete && report.rows[0].success && report.rows[0].valid);
    let q = &report.rows[0].queries;
    for (cap, n) in [
        (Capability::Character, 1),
        (Capability::Triggers, 1),
        (Capability::Locale, 1),
        (Capability::Localize, 2),
        (Capability::Other, 1),
    ] {
        assert_eq!(q.calls[cap as usize], [n; 4]);
        assert_eq!(q.failed[cap as usize], [0; 4]);
        assert_ne!(q.source_masks[cap as usize], 0);
    }
    assert_eq!(q.calls[Capability::Navigation as usize], [0; 4]);
}
#[test]
fn query_diagnostic_preserves_parse_native_and_materialization_failure_atomicity() {
    for (stage, expression) in [
        (1, "physics.triggerEvents(1)"),
        (
            2,
            "physics.computeCharacterMotion({entity_id:table,translation:[1,0,0]})",
        ),
        (3, "physics.triggerEvents()"),
    ] {
        let mut world = NativeWorld::new(scene());
        let mut control = NativeWorld::new(scene());
        let mut script = host(&format!(
            "chunks[0].transforms[0]=42;commands.setVelocity(entity,[2,0,0]);try{{{expression}}}catch{{}}"
        ));
        if stage == 3 {
            queries::FAIL_RESULT_ALLOCATION.with(|flag| flag.set(true));
        }
        let (result, report) = recorded(&mut script, &mut world);
        assert!(result.is_err());
        assert_same_after_next_step(&mut world, &mut control);
        assert!(!report.complete && !report.rows[0].success);
        let cap = if stage == 2 {
            Capability::Character
        } else {
            Capability::Triggers
        } as usize;
        let q = &report.rows[0].queries;
        for i in 0..4 {
            assert_eq!(q.calls[cap][i], u32::from(i <= stage));
        }
        assert_eq!(q.failed[cap][0], 1);
        assert_eq!(q.failed[cap][stage], 1);
        assert!(!queries::FAIL_RESULT_ALLOCATION.with(|flag| flag.replace(false)));
    }
}
#[test]
fn query_diagnostic_counts_recursive_getters_without_changing_their_behavior() {
    let mut world = NativeWorld::new(scene());
    let mut control = NativeWorld::new(scene());
    let (result, report) = recorded(
        &mut host(
            r#"
        const value=localization.localize({table_id:table,key:'hello',get arguments(){
            if(localization.locale().locale!=='en')throw Error('inner');return {name:'Ada'};
        }}); if(value.text!=='Hello Ada')throw Error('outer');
    "#,
        ),
        &mut world,
    );
    result.unwrap();
    assert_same_after_next_step(&mut world, &mut control);
    assert!(!report.complete && report.rows[0].success && !report.rows[0].valid);
    assert_eq!(
        report.rows[0].queries.recursive_calls[Capability::Locale as usize],
        1
    );
    assert_eq!(
        report.rows[0].queries.calls[Capability::Locale as usize],
        [1; 4]
    );
    assert_eq!(
        report.rows[0].queries.calls[Capability::Localize as usize],
        [1; 4]
    );
}
#[test]
fn query_diagnostic_keeps_admission_refusals_and_guest_call_limits_exact() {
    let mut world = NativeWorld::new(scene());
    let mut control = NativeWorld::new(scene());
    let (result, report) = recorded(
        &mut host(
            "chunks[0].transforms[0]=42;for(let i=0;i<65;i++){try{localization.locale()}catch{}}",
        ),
        &mut world,
    );
    assert!(matches!(result, Err(ScriptError::InputLimit)));
    assert_same_after_next_step(&mut world, &mut control);
    let q = &report.rows[0].queries;
    assert_eq!(q.calls[Capability::Locale as usize], [65, 64, 64, 64]);
    assert_eq!(q.failed[Capability::Locale as usize], [1, 0, 0, 0]);
}
#[test]
fn query_diagnostic_observes_expired_capability_without_reactivating_it() {
    let mut world = NativeWorld::new(scene());
    let mut control = NativeWorld::new(scene());
    let mut script = host(
        "if(globalThis.oldLocale){let refused=false;try{oldLocale()}catch{refused=true;}if(!refused)throw Error('expired');}globalThis.oldLocale=localization.locale;if(localization.locale().locale!=='en')throw Error('current');",
    );
    // Retain a real previous-tick function, then observe its refusal and the new
    // capability's successful result in the same callback and committed world.
    script.tick(&mut world, 1. / 60.).unwrap();
    let (result, report) = recorded(&mut script, &mut world);
    result.unwrap();
    assert_same_after_next_step(&mut world, &mut control);
    assert!(report.complete && report.rows[0].valid);
    let q = &report.rows[0].queries;
    assert_eq!(q.calls[Capability::Locale as usize], [2, 1, 1, 1]);
    assert_eq!(q.failed[Capability::Locale as usize], [1, 0, 0, 0]);
}
#[test]
fn query_diagnostic_records_native_path_hit_and_null_as_successful_queries() {
    let mut world = NativeWorld::new(navigation::scene());
    let mut control = NativeWorld::new(navigation::scene());
    let expected = control
        .navigation_path(navigation::id(1), &navigation::path())
        .unwrap();
    let (result, report) = recorded(
        &mut host(
            r#"
        const owner='00000000000000000000000001',q={start:[-8,0,0],end:[8,0,0],snap_distance:1,max_visited:1000};
        const p=navigation.findPath(owner,q);if(p.points.length<=2)throw Error('detour');
        if(navigation.findPath(owner,{...q,start:[200,0,0]})!==null)throw Error('miss');
        p.points[0][0]=999;
    "#,
        ),
        &mut world,
    );
    result.unwrap();
    assert!(report.complete);
    assert_eq!(
        world
            .navigation_path(navigation::id(1), &navigation::path())
            .unwrap(),
        expected
    );
    assert_same_after_next_step(&mut world, &mut control);
    assert_eq!(
        world
            .navigation_path(navigation::id(1), &navigation::path())
            .unwrap(),
        control
            .navigation_path(navigation::id(1), &navigation::path())
            .unwrap()
    );
    assert_eq!(
        report.rows[0].queries.calls[Capability::Navigation as usize],
        [2; 4]
    );
    assert_eq!(
        report.rows[0].queries.failed[Capability::Navigation as usize],
        [0; 4]
    );
    #[cfg(feature = "diagnostic-navigation")]
    assert_eq!(report.rows[0].navigation.failed[0], 1); // optional null path, not query error
}
