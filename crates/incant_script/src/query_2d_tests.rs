use super::*;
#[path = "../../incant_runtime/tests/support/physics_queries_2d.rs"]
mod fixture;
use crate::tests::check;
use fixture::*;
const QUERY: &str = "{origin:[0.5,3],direction:[0,-2],max_distance:10,include_sensors:false,exclude_entity:'00000000000000000000000002',memberships:4294967295,filter:4294967295}";
fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(
        CompiledScript::compile(&format!(
            "exports.default={{update({{chunks,commands,physics}}){{const query={QUERY};{body}}}}};"
        ))
        .unwrap(),
        Duration::from_secs(2),
    )
    .unwrap()
}
fn world() -> NativeWorld {
    NativeWorld::new(scene())
}
const WRITE: &str = "for(const c of chunks){const i=c.ids.indexOf('00000000000000000000000002');if(i>=0)c.transforms[i*10]=40;}commands.despawn('00000000000000000000000003');";

#[test]
fn planar_ground_tiles_misses_sensor_masks_and_owned_xy_hits() {
    let mut world = world();
    let mut host = host(
        r#"
        const hit=physics.raycast2D(query);
        if(hit.entity_id!=='00000000000000000000000001'||hit.distance!==2||hit.point.length!==2||hit.point[1]!==1||hit.normal[1]!==1)throw Error('ground');
        if(physics.raycast2D({...query,origin:[1.5,3]})!==null)throw Error('gap');
        if(physics.raycast2D({...query,filter:0})!==null||physics.raycast2D({...query,memberships:0})!==null)throw Error('mask');
        if(physics.raycast2D({...query,include_sensors:true}).entity_id!=='00000000000000000000000003')throw Error('sensor');
        if(physics.raycast2D({...query,exclude_entity:null}).entity_id!=='00000000000000000000000002')throw Error('exclude');
        if(physics.raycast2D({...query,origin:[0.5,0.5]}).distance!==0)throw Error('solid origin');
        hit.point[0]=777;hit.entity_id='bad';if(physics.raycast2D(query).point[0]!==0.5)throw Error('aliased');
        for(const c of chunks){const i=c.ids.indexOf('00000000000000000000000001');if(i>=0)c.transforms[i*10+1]=100;}
        if(physics.raycast2D(query).distance!==2)throw Error('pending numeric');
    "#,
    );
    host.tick(&mut world, 1. / 64.).unwrap();
    world.step().unwrap();
    assert!(world.raycast_2d(&ray()).unwrap().is_none());
}

#[test]
fn planar_events_track_committed_enter_exit_and_owned_nondraining_results() {
    let mut world = world();
    let mut empty = host("if(physics.triggerEvents2D().length)throw Error('unexpected events');");
    empty.tick(&mut world, 1. / 64.).unwrap();
    world.step().unwrap();
    let mut enter = host(
        r#"
        const events=physics.triggerEvents2D();const e=events[0];
        if(events.length!==1||e.first!=='00000000000000000000000002'||e.second!=='00000000000000000000000003'||!e.entered)throw Error('enter');
        e.entered=false;events.length=0;if(!physics.triggerEvents2D()[0].entered)throw Error('alias/drain');
    "#,
    );
    enter.tick(&mut world, 1. / 64.).unwrap();
    world.step().unwrap();
    empty.tick(&mut world, 1. / 64.).unwrap();
    let mut move_away = host(
        "for(const c of chunks){const i=c.ids.indexOf('00000000000000000000000002');if(i>=0)c.transforms[i*10]=10;}if(physics.triggerEvents2D().length)throw Error('speculative');",
    );
    move_away.tick(&mut world, 1. / 64.).unwrap();
    world.step().unwrap();
    host("const e=physics.triggerEvents2D()[0];if(!e||e.entered||e.first!=='00000000000000000000000002')throw Error('exit');").tick(&mut world,1./64.).unwrap();
}

#[test]
fn retained_planar_capabilities_expire_and_materialization_bypasses_guest_setters() {
    let mut world = world();
    world.step().unwrap();
    let mut host = host(
        r#"
        if(globalThis.oldRay){for(const call of [()=>oldRay(query),()=>oldEvents()]){let refused=false;try{call()}catch{refused=true}if(!refused)throw Error('old cap');}}
        globalThis.oldRay=physics.raycast2D;globalThis.oldEvents=physics.triggerEvents2D;
        Object.defineProperty(Array.prototype,'0',{set(){throw Error('array setter')},configurable:true});
        Object.defineProperty(Object.prototype,'entity_id',{set(){throw Error('object setter')},configurable:true});
        const hit=physics.raycast2D(query), events=physics.triggerEvents2D();
        delete Array.prototype[0];delete Object.prototype.entity_id;
        if(hit.distance!==2||events.length!==1)throw Error('results');
    "#,
    );
    host.tick(&mut world, 1. / 64.).unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    check(
        &host,
        &format!(
            "for(const call of [()=>oldRay({QUERY}),()=>oldEvents()]){{let refused=false;try{{call()}}catch{{refused=true}}if(!refused)throw Error('escaped');}}"
        ),
    );
}

#[test]
fn caught_planar_errors_roll_back_numeric_and_structural_writes_and_restore_solver() {
    for bad in [
        "physics.raycast2D({...query,direction:[0,0]})",
        "physics.raycast2D({...query,origin:[0,0,0]})",
        "physics.raycast2D({...query,origin:[NaN,0]})",
        "physics.raycast2D({...query,max_distance:Infinity})",
        "physics.raycast2D({...query,include_sensors:1})",
        "physics.raycast2D({...query,filter:0.5})",
        "physics.raycast2D({...query,exclude_entity:'bad'})",
        "physics.raycast2D({...query,scene_id:'other'})",
        "physics.raycast2D({...query,get origin(){throw Error('getter')}})",
        "physics.triggerEvents2D(1)",
    ] {
        let mut world = world();
        let mut control = self::world();
        let before = world.snapshot();
        let mut host = host(&format!("{WRITE}try{{{bad}}}catch{{}}"));
        assert!(host.tick(&mut world, 1. / 64.).is_err(), "{bad}");
        assert_eq!(world.snapshot(), before, "{bad}");
        assert!(matches!(
            host.tick(&mut world, 1. / 64.),
            Err(ScriptError::FailedSession)
        ));
        world.step().unwrap();
        control.step().unwrap();
        assert_eq!(
            world.physics_2d_state(id(2)),
            control.physics_2d_state(id(2))
        );
        assert_eq!(world.physics_state(id(4)), control.physics_state(id(4)));
        assert_eq!(world.physics_2d_events(), control.physics_2d_events());
    }
}

#[test]
fn two_and_three_dimensional_calls_share_one_budget_including_caught_recursive_getters() {
    for tail in [
        "for(let i=0;i<256;i++){physics.triggerEvents();physics.triggerEvents2D();physics.raycast2D(query);physics.raycast({origin:[0,0,0],direction:[1,0,0],max_distance:10,include_sensors:false,memberships:4294967295,filter:4294967295});}try{physics.triggerEvents2D()}catch{}",
        "Object.defineProperty(query,'origin',{get(){for(let i=0;i<1024;i++){try{physics.triggerEvents2D()}catch{}}return [0.5,3]}});try{physics.raycast2D(query)}catch{}",
    ] {
        let mut world = world();
        let before = world.snapshot();
        let mut host = host(&format!("{WRITE}{tail}"));
        assert!(matches!(
            host.tick(&mut world, 1. / 64.),
            Err(ScriptError::InputLimit)
        ));
        assert_eq!(world.snapshot(), before);
    }
    let mut world = world();
    host("for(let i=0;i<512;i++){physics.triggerEvents2D();physics.triggerEvents();}")
        .tick(&mut world, 1. / 64.)
        .unwrap();
}

#[test]
fn nested_valid_getter_and_caught_materialization_failure_keep_loan_rules() {
    let mut world = world();
    host("Object.defineProperty(query,'origin',{get(){if(physics.triggerEvents2D().length)throw Error('nested');return [0.5,3]}});if(physics.raycast2D(query).distance!==2)throw Error('hit');").tick(&mut world,1./64.).unwrap();
    for (events, fault) in [
        (false, &queries::FAIL_HIT_ALLOCATION),
        (true, &queries::FAIL_RESULT_ALLOCATION),
    ] {
        let mut world = self::world();
        let before = world.snapshot();
        let operation = if events {
            "physics.triggerEvents2D()"
        } else {
            "physics.raycast2D(query)"
        };
        let mut host = host(&format!("{WRITE}try{{{operation}}}catch{{}}"));
        fault.with(|flag| flag.set(true));
        let result = host.tick(&mut world, 1. / 64.);
        assert!(!fault.with(|flag| flag.replace(false)));
        assert!(matches!(result, Err(ScriptError::MemoryAllocation)));
        assert_eq!(world.snapshot(), before);
        world.step().unwrap();
        assert!(world.physics_2d_state(id(2)).is_some());
    }
}
