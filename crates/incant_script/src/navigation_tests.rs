use super::*;
#[path = "../../incant_runtime/tests/support/navigation_queries.rs"]
pub(super) mod fixture;
use fixture::*;
const OWNER: &str = "00000000000000000000000001";
const MESH: &str = "{start:[-8,0,0],end:[8,0,0],snap_distance:1,max_visited:1000}";
const GRID: &str = "{start:[0,0],end:[2,0],diagonal:true,max_expansions:100}";
const WRITE: &str = "for(const c of chunks){const i=c.ids.indexOf('00000000000000000000000004');if(i>=0)c.transforms[i*10]=40;}commands.setVelocity('00000000000000000000000004',[7,0,0]);";
fn host(body: &str) -> ScriptHost {
    ScriptHost::with_budget(CompiledScript::compile(&format!(
        "exports.default={{update({{chunks,commands,physics,navigation,timers}}){{const owner='{OWNER}',mesh={MESH},grid={GRID};{body}}}}};"
    )).unwrap(), Duration::from_secs(3)).unwrap()
}
fn world() -> NativeWorld {
    NativeWorld::new(scene())
}
#[test]
fn mesh_detour_grid_cost_misses_owned_values_and_movement_are_observable() {
    let mut world = world();
    host(r#"
        const p=navigation.findPath(owner,mesh),g=navigation.findGridPath(owner,grid);
        if(p.points.length<=2||p.generation<=0||p.visited<=0||p.corridor.length===0||p.traversals.length!==0)throw Error('detour');
        if(g.cells.length!==5||g.cost!==4000||g.cells[1][1]!==1||g.generation<=0)throw Error('grid');
        if(navigation.findPath(owner,{...mesh,start:[200,0,0]})!==null)throw Error('snap miss');
        if(navigation.findGridPath(owner,{...grid,end:[1,0]})!==null)throw Error('blocked');
        p.points[0][0]=999;p.corridor.length=0;g.cells[0][0]=999;
        if(navigation.findPath(owner,mesh).points[0][0]===999||navigation.findGridPath(owner,grid).cells[0][0]!==0)throw Error('alias');
        commands.setVelocity('00000000000000000000000004',[g.cells[1][0],g.cells[1][1],0]);
    "#).tick(&mut world,1./60.).unwrap();
    // Guest result mutations must leave the retained native route unchanged.
    assert_eq!(
        world.grid_path(id(1), &grid()).unwrap().unwrap().cells,
        vec![[0, 0], [0, 1], [1, 1], [2, 1], [2, 0]]
    );
    world.step().unwrap();
    assert_eq!(
        world.inspect(id(4)).unwrap().transform.translation,
        [0., 1. / 60., 0.]
    );
}
#[test]
fn retained_navigation_functions_expire_but_owned_results_survive_and_setters_are_bypassed() {
    let mut world = world();
    let mut host = host(
        r#"
        if(globalThis.old){for(const f of old){let refused=false;try{f(owner,grid)}catch{refused=true}if(!refused)throw Error('old')};if(saved.cells.length!==5)throw Error('owned');}
        const nested={...mesh,get start(){if(navigation.findGridPath(owner,grid).cost!==4000)throw Error('nested');return mesh.start}};
        if(navigation.findPath(owner,nested).points.length<=2)throw Error('nested path');
        globalThis.old=[navigation.findPath,navigation.findGridPath];
        Object.defineProperty(Array.prototype,'0',{set(){throw Error('setter')},configurable:true});
        Object.defineProperty(Object.prototype,'points',{set(){throw Error('setter')},configurable:true});
        const p=navigation.findPath(owner,mesh);globalThis.saved=navigation.findGridPath(owner,grid);
        delete Array.prototype[0];delete Object.prototype.points;
        if(p.points.length<=2)throw Error('missing');
    "#,
    );
    for _ in 0..2 {
        host.tick(&mut world, 1. / 60.).unwrap();
    }
    tests::check(
        &host,
        &format!(
            "for(const f of old){{let refused=false;try{{f('{OWNER}',{GRID})}}catch{{refused=true}}if(!refused)throw Error('escaped')}}"
        ),
    );
}
#[test]
fn native_query_refusals_poison_caught_errors_and_roll_back_numeric_and_structural_state() {
    let cooked = scene();
    for bad in [
        "navigation.findPath('bad',mesh)",
        "navigation.findPath(owner,{...mesh,start:[NaN,0,0]})",
        "navigation.findPath(owner,{...mesh,start:[100000.000001,0,0]})",
        "navigation.findPath(owner,{...mesh,snap_distance:10.000000001})",
        "navigation.findPath(owner,{...mesh,max_visited:1.5})",
        "navigation.findPath(owner,{...mesh,max_visited:0})",
        "navigation.findPath(owner,{...mesh,scene_id:owner})",
        "navigation.findPath(owner,{...mesh,get start(){throw Error('getter')}})",
        "navigation.findGridPath(owner,{...grid,start:[-1,0]})",
        "navigation.findGridPath(owner,{...grid,start:[65536,0]})",
        "navigation.findGridPath(owner,{...grid,start:[0.5,0]})",
        "navigation.findGridPath(owner,{...grid,start:[0,0,0]})",
        "navigation.findGridPath(owner,{...grid,end:[3,0]})",
        "navigation.findGridPath(owner,{...grid,diagonal:1})",
        "navigation.findGridPath(owner,{...grid,max_expansions:1})",
        "navigation.findGridPath(owner,{...grid,max_expansions:Infinity})",
        "navigation.findGridPath('00000000000000000000000004',grid)",
        "navigation.findGridPath(owner,grid,1)",
    ] {
        let mut world = NativeWorld::new(cooked.clone());
        let before = world.snapshot();
        let original = world.navigation_path(id(1), &path()).unwrap();
        let mut host = host(&format!(
            "{WRITE}navigation.findGridPath(owner,grid);try{{{bad}}}catch{{}}"
        ));
        assert!(host.tick(&mut world, 1. / 60.).is_err(), "{bad}");
        assert_eq!(world.snapshot(), before, "{bad}");
        assert_eq!(world.navigation_path(id(1), &path()).unwrap(), original);
        assert!(matches!(
            host.tick(&mut world, 1. / 60.),
            Err(ScriptError::FailedSession)
        ));
    }
}
#[test]
fn call_and_total_requested_expansion_budgets_include_nested_getters_and_physics() {
    let cooked = scene();
    for tail in [
        "for(let i=0;i<33;i++){try{navigation.findGridPath(owner,{start:[0,0],end:[0,0],max_expansions:1})}catch{}}",
        "navigation.findPath(owner,{...mesh,max_visited:32768});navigation.findGridPath(owner,{...grid,max_expansions:32768});try{navigation.findGridPath(owner,grid)}catch{}",
        "navigation.findGridPath(owner,{start:[0,0],end:[0,0]});try{navigation.findGridPath(owner,grid)}catch{}",
        "for(let i=0;i<1024;i++)physics.triggerEvents();try{navigation.findGridPath(owner,grid)}catch{}",
        "try{navigation.findPath(owner,{...mesh,get start(){for(let i=0;i<33;i++){try{navigation.findGridPath(owner,{start:[0,0],end:[0,0],max_expansions:1})}catch{}}return mesh.start}})}catch{}",
    ] {
        let mut world = NativeWorld::new(cooked.clone());
        let before = world.snapshot();
        assert!(
            matches!(
                host(&format!("{WRITE}{tail}")).tick(&mut world, 1. / 60.),
                Err(ScriptError::InputLimit)
            ),
            "{tail}"
        );
        assert_eq!(world.snapshot(), before);
    }
    let mut world = world();
    host("navigation.findPath(owner,{...mesh,max_visited:32768});navigation.findGridPath(owner,{...grid,max_expansions:32768});").tick(&mut world,1./60.).unwrap();
    host("for(let i=0;i<32;i++)navigation.findGridPath(owner,{start:[0,0],end:[0,0],max_expansions:1});").tick(&mut world,1./60.).unwrap();
}
#[test]
fn pending_numeric_source_motion_is_invisible_then_commits_and_late_controls_roll_back() {
    let mut world = world();
    host(r#"
        globalThis.g=navigation.findPath(owner,mesh).generation;
        for(const c of chunks){const i=c.ids.indexOf('00000000000000000000000003');if(i>=0)c.transforms[i*10+2]=4;}
        const p=navigation.findPath(owner,mesh);if(p.points.length<=2||p.generation!==g)throw Error('speculative');
    "#).tick(&mut world,1./60.).unwrap();
    assert_eq!(
        world
            .navigation_path(id(1), &path())
            .unwrap()
            .unwrap()
            .points
            .len(),
        2
    );
    let before = world.snapshot();
    let path_before = world.navigation_path(id(1), &path()).unwrap();
    assert!(host(&format!("{WRITE}navigation.findPath(owner,mesh);commands.setVelocity('00000000000000000000000002',[1,0,0]);")).tick(&mut world,1./60.).is_err());
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.navigation_path(id(1), &path()).unwrap(), path_before);
}
#[test]
fn off_mesh_crossings_keep_exact_link_identity_indices_direction_and_disabled_miss() {
    let mut world = NativeWorld::new(linked_scene(true));
    host(r#"
        const p=navigation.findPath(owner,mesh),t=p.traversals[0];
        if(p.traversals.length!==1||t.link_id!=='00000000000000000000000008'||t.to_index!==t.from_index+1||t.reversed||p.points[t.from_index][0]>=0||p.points[t.to_index][0]<=0)throw Error('link');
        if(!navigation.findPath(owner,{...mesh,start:mesh.end,end:mesh.start}).traversals[0].reversed)throw Error('reverse');
        t.link_id='bad';if(navigation.findPath(owner,mesh).traversals[0].link_id==='bad')throw Error('alias');
    "#).tick(&mut world,1./60.).unwrap();
    let mut world = NativeWorld::new(linked_scene(false));
    host("if(navigation.findPath(owner,mesh)!==null)throw Error('disconnected');")
        .tick(&mut world, 1. / 60.)
        .unwrap();
}
#[test]
fn caught_materialization_failure_restores_cache_and_all_immediate_writes() {
    let mut world = world();
    let before = world.snapshot();
    let expected = world.navigation_path(id(1), &path()).unwrap();
    let mut host = host(&format!(
        "{WRITE}try{{navigation.findGridPath(owner,grid)}}catch{{}}"
    ));
    queries::FAIL_RESULT_ALLOCATION.with(|flag| flag.set(true));
    assert!(matches!(
        host.tick(&mut world, 1. / 60.),
        Err(ScriptError::MemoryAllocation)
    ));
    assert!(!queries::FAIL_RESULT_ALLOCATION.with(|flag| flag.replace(false)));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.navigation_path(id(1), &path()).unwrap(), expected);
}

#[test]
fn authored_sources_share_navigation_quota_and_one_atomic_transaction() {
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
        let b = if reject {
            "try{navigation.findGridPath(owner,{...grid,max_expansions:32769})}catch{}"
        } else {
            "navigation.findGridPath(owner,{...grid,max_expansions:32768});commands.setVelocity('00000000000000000000000004',[1,0,0]);"
        };
        let source = format!(
            "exports.default={{sources:[{{id:'0000000000000000000000000A',behavior:{{update({{chunks,commands,navigation}}){{const owner='{OWNER}',grid={GRID};{WRITE}navigation.findGridPath(owner,{{...grid,max_expansions:32768}});}}}}}},{{id:'0000000000000000000000000B',behavior:{{update({{navigation,commands}}){{const owner='{OWNER}',grid={GRID};{b}}}}}}}]}};"
        );
        let code =
            CompiledScript::compile_project("navigation.cjs", &source, bindings.clone()).unwrap();
        let mut host =
            ScriptHost::initialize(code, Duration::from_secs(3), Some(&world), None).unwrap();
        let result = host.tick(&mut world, 1. / 60.);
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

#[test]
fn output_generation_never_rounds_across_javascript_integer_precision() {
    let runtime = rquickjs::Runtime::new().unwrap();
    let context = rquickjs::Context::full(&runtime).unwrap();
    context.with(|ctx| {
        let good = incant_runtime::GridPath {
            cells: vec![[0, 0]],
            cost: 0,
            generation: 9_007_199_254_740_991,
        };
        let value = navigation_values::grid_value(&ctx, Some(good.clone())).unwrap();
        assert_eq!(
            value
                .as_object()
                .unwrap()
                .get::<_, f64>("generation")
                .unwrap(),
            good.generation as f64
        );
        let mut bad = good;
        bad.generation += 1;
        assert!(navigation_values::grid_value(&ctx, Some(bad)).is_err());
    });
}
