use super::{navigation_tests::fixture::*, *};

const SETUP: &str = r#"
const owner='00000000000000000000000001', moving='00000000000000000000000004';
const agent=(id=moving)=>({id,position:[0,0,0],velocity:[0,0],preferred_velocity:[1,0],radius:.25,height:1.8,max_speed:2,responsibility:1});
const q={agents:[agent()],time_horizon:2,obstacle_time_horizon:2};
"#;
const WRITE: &str = "for(const c of chunks){const i=c.ids.indexOf(moving);if(i>=0)c.transforms[i*10]=40;}commands.setVelocity(moving,[7,0,0]);";
fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(
        CompiledScript::compile(&format!(
            "exports.default={{update({{chunks,commands,navigation,physics}}){{{SETUP}{body}}}}};"
        ))
        .unwrap(),
        Duration::from_secs(3),
    )
    .unwrap()
}

#[test]
fn owned_bulk_proposals_feed_actual_native_movement_without_json() {
    let mut world = NativeWorld::new(scene());
    let mut script = host(
        r#"
        JSON.stringify=JSON.parse=()=>{throw Error('no JSON');};
        const out=navigation.steerAgents(q);
        if(out.length!==1||out[0].id!==moving||out[0].velocity[0]!==1||out[0].velocity[1]!==0||out[0].neighbors!==0)throw Error('free motion');
        out[0].velocity[0]=100;out[0].id='bad';
        q.agents[0].position=new Float64Array([0,0,0]);
        q.agents[0].velocity=new Float64Array([0,0]);
        const again=navigation.steerAgents(q)[0];
        commands.setVelocity(moving,[again.velocity[0],0,again.velocity[1]]);
    "#,
    );
    script.tick(&mut world, 1. / 60.).unwrap();
    world.step().unwrap();
    assert_eq!(
        world.inspect(id(4)).unwrap().transform.translation,
        [1. / 60., 0., 0.]
    );
}

#[test]
fn callbacks_expire_owned_outputs_survive_and_nested_getters_do_not_borrow_native_state() {
    let mut world = NativeWorld::new(scene());
    let mut script = host(
        r#"
        if(globalThis.old){let refused=false;try{old(q)}catch{refused=true}if(!refused)throw Error('expired');if(saved[0].id!==moving)throw Error('owned');}
        const nested={...q,get agents(){if(navigation.findGridPath(owner,{start:[0,0],end:[0,0],max_expansions:1}).cells.length!==1)throw Error('nested');return q.agents;}};
        Object.defineProperty(Array.prototype,'0',{set(){throw Error('inherited setter')},configurable:true});
        Object.defineProperty(Object.prototype,'velocity',{set(){throw Error('inherited setter')},configurable:true});
        globalThis.saved=navigation.steerAgents(nested);globalThis.old=navigation.steerAgents;
        delete Array.prototype[0];delete Object.prototype.velocity;
    "#,
    );
    script.tick(&mut world, 1. / 60.).unwrap();
    script.tick(&mut world, 1. / 60.).unwrap();
}

#[test]
fn malformed_caught_queries_reject_all_native_writes_and_fail_the_host() {
    for bad in [
        "navigation.steerAgents()",
        "navigation.steerAgents(q,1)",
        "navigation.steerAgents({...q,scene_id:owner})",
        "navigation.steerAgents({...q,time_horizon:1/60-1e-10})",
        "navigation.steerAgents({...q,time_horizon:10.000000001})",
        "navigation.steerAgents({...q,obstacle_time_horizon:NaN})",
        "navigation.steerAgents({...q,margin:1.000000001})",
        "navigation.steerAgents({...q,agents:[agent(),agent()]})",
        "navigation.steerAgents({...q,agents:[{...agent(),id:'bad'}]})",
        "navigation.steerAgents({...q,agents:[{...agent(),position:[10000.000000001,0,0]}]})",
        "navigation.steerAgents({...q,agents:[{...agent(),velocity:[30.000000001,0]}]})",
        "navigation.steerAgents({...q,agents:[{...agent(),preferred_velocity:[Infinity,0]}]})",
        "navigation.steerAgents({...q,agents:[{...agent(),radius:.049999999999}]})",
        "navigation.steerAgents({...q,agents:[{...agent(),responsibility:10.000000001}]})",
        "navigation.steerAgents({...q,agents:[{...agent(),max_speed:-1e-10}]})",
        "navigation.steerAgents({...q,agents:Array(129).fill(agent())})",
        "navigation.steerAgents({...q,obstacles:null})",
        "navigation.steerAgents({...q,obstacles:[{id:owner,closed:false,vertices:[[0,0]],min_y:0,max_y:2}]})",
        "navigation.steerAgents({...q,obstacles:[{id:owner,closed:true,vertices:[[0,0],[1,0],[0,0]],min_y:0,max_y:2}]})",
        "navigation.steerAgents({...q,get agents(){throw Error('getter')}})",
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut script = host(&format!("{WRITE}try{{{bad}}}catch{{}}"));
        assert!(script.tick(&mut world, 1. / 60.).is_err(), "{bad}");
        assert_eq!(world.snapshot(), before, "{bad}");
        assert!(matches!(
            script.tick(&mut world, 1. / 60.),
            Err(ScriptError::FailedSession)
        ));
    }
}

#[test]
fn requested_work_and_call_limits_charge_before_caught_or_recursive_execution() {
    let many =
        "Array.from({length:64},(_,i)=>agent(i.toString(16).toUpperCase().padStart(26,'0')))";
    for tail in [
        "for(let i=0;i<33;i++){try{navigation.steerAgents({...q,agents:[]})}catch{}}".to_owned(),
        format!("navigation.steerAgents({{...q,agents:{many}}});navigation.steerAgents({{...q,agents:{many}}});try{{navigation.steerAgents(q)}}catch{{}}"),
        "for(let i=0;i<1024;i++)physics.triggerEvents();try{navigation.steerAgents(q)}catch{}".to_owned(),
        "try{navigation.steerAgents({...q,get agents(){for(let i=0;i<32;i++){try{navigation.steerAgents({...q,agents:[]})}catch{}}return q.agents;}})}catch{}".to_owned(),
    ] {
        let mut world=NativeWorld::new(scene()); let before=world.snapshot();
        let result=host(&format!("{WRITE}{tail}")).tick(&mut world,1./60.);
        assert!(matches!(result,Err(ScriptError::InputLimit)),"{tail}: {result:?}");
        assert_eq!(world.snapshot(),before);
    }
    let mut world = NativeWorld::new(scene());
    host(&format!("navigation.steerAgents({{...q,agents:{many}}});navigation.steerAgents({{...q,agents:{many}}});")).tick(&mut world,1./60.).unwrap();
}

#[test]
fn caught_result_allocation_failure_rolls_back_native_values_and_commands() {
    let mut world = NativeWorld::new(scene());
    let before = world.snapshot();
    let mut script = host(&format!("{WRITE}try{{navigation.steerAgents(q)}}catch{{}}"));
    queries::FAIL_RESULT_ALLOCATION.with(|flag| flag.set(true));
    assert!(matches!(
        script.tick(&mut world, 1. / 60.),
        Err(ScriptError::MemoryAllocation)
    ));
    assert!(!queries::FAIL_RESULT_ALLOCATION.with(|flag| flag.replace(false)));
    assert_eq!(world.snapshot(), before);
}

#[test]
fn horizontal_neighbors_and_directed_obstacles_change_owned_proposals() {
    let mut world = NativeWorld::new(scene());
    host(r#"
        const pair=navigation.steerAgents({...q,agents:[
          {...agent(moving),position:[1,0,0],preferred_velocity:[-1,0]},
          {...agent(owner),position:[-1,0,0]},
        ]});
        if(pair[0].id!==owner||pair[1].id!==moving||pair.some(a=>a.neighbors!==1||Math.hypot(...a.velocity)>2.000001))throw Error('pair');
        if(pair[0].velocity[0]===1&&pair[0].velocity[1]===0)throw Error('no avoidance');
        const wall={id:owner,closed:false,vertices:[new Float64Array([1,2]),[1,-2]],min_y:0,max_y:2};
        const blocked=navigation.steerAgents({...q,obstacles:[wall]})[0];
        if(blocked.neighbors!==0||blocked.velocity[0]>=1||!Number.isFinite(blocked.velocity[1]))throw Error('wall');
        const above=navigation.steerAgents({...q,obstacles:[{...wall,min_y:3,max_y:4}]})[0];
        if(above.velocity[0]!==1||above.velocity[1]!==0)throw Error('height');
    "#).tick(&mut world,1./60.).unwrap();
}

#[test]
fn obstacle_edges_are_bounded_across_calls_including_empty_agent_groups() {
    let polygons = r#"
      const obstacles=Array.from({length:4},(_,i)=>({
        id:i.toString().padStart(26,'0'),closed:true,min_y:0,max_y:2,
        vertices:Array.from({length:32},(_,j)=>[Math.cos(j*Math.PI/16),Math.sin(j*Math.PI/16)]),
      }));
      navigation.steerAgents({...q,agents:[],obstacles});
    "#;
    host(polygons)
        .tick(&mut NativeWorld::new(scene()), 1. / 60.)
        .unwrap();
    let mut world = NativeWorld::new(scene());
    let before = world.snapshot();
    let mut script = host(&format!(
        r#"{WRITE}{polygons}
      try{{navigation.steerAgents({{...q,agents:[],obstacles:[{{id:owner,closed:false,vertices:[[0,0],[1,0]],min_y:0,max_y:2}}]}})}}catch{{}}
    "#
    ));
    assert!(matches!(
        script.tick(&mut world, 1. / 60.),
        Err(ScriptError::InputLimit)
    ));
    assert_eq!(world.snapshot(), before);
}

#[test]
fn authored_sources_share_agent_budget_and_rollback_as_one_transaction() {
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;
    let cooked = scene();
    let bindings = ScriptBindings::new(
        Sha256::digest(cooked.to_bytes()).into(),
        vec![id(10), id(11)],
        vec![
            ScriptInstance {
                entity: id(1),
                source: id(10),
                props: BTreeMap::new(),
            },
            ScriptInstance {
                entity: id(4),
                source: id(11),
                props: BTreeMap::new(),
            },
        ],
    )
    .unwrap();
    for reject in [false, true] {
        let mut world = NativeWorld::new(cooked.clone());
        let before = world.snapshot();
        let first = format!(
            "{SETUP}{WRITE}navigation.steerAgents({{...q,agents:Array.from({{length:64}},(_,i)=>agent(i.toString(16).toUpperCase().padStart(26,'0')))}});"
        );
        let second = format!(
            "{SETUP}try{{navigation.steerAgents({{...q,agents:Array.from({{length:{}}},(_,i)=>agent(i.toString(16).toUpperCase().padStart(26,'0')))}})}}catch{{}}commands.setVelocity(moving,[1,0,0]);",
            if reject { 65 } else { 64 }
        );
        let source = format!(
            "exports.default={{sources:[{{id:'0000000000000000000000000A',behavior:{{update({{chunks,commands,navigation}}){{{first}}}}}}},{{id:'0000000000000000000000000B',behavior:{{update({{commands,navigation}}){{{second}}}}}}}]}};"
        );
        let code =
            CompiledScript::compile_project("steering.cjs", &source, bindings.clone()).unwrap();
        let mut script =
            ScriptHost::initialize(code, Duration::from_secs(3), Some(&world), None).unwrap();
        let result = script.tick(&mut world, 1. / 60.);
        if reject {
            assert!(matches!(result, Err(ScriptError::InputLimit)));
            assert_eq!(world.snapshot(), before);
        } else {
            result.unwrap();
            world.step().unwrap();
            assert_eq!(
                world.inspect(id(4)).unwrap().transform.translation[0],
                40. + 1. / 60.
            );
        }
    }
}
