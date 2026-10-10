use incant_cmd::{Actor, Command, CommandBus};
use incant_core::{Engine, GridNavigationQuery, GridPathRequest};
use incant_doc::{Entity, NavigationGrid, Project, Scene};
use incant_script::{PlaySession, ScriptHost};
use serde_json::json;

fn fixture() -> (Project, String, String) {
    let mut project = Project::empty("Grid navigation");
    let mut scene = Scene::new("Tiles");
    let mut entity = Entity::new("Costs");
    entity.components.insert(
        "NavigationGrid".into(),
        json!(NavigationGrid {
            dimensions: [3, 3],
            costs: vec![1, 1, 1, 1, 100, 1, 1, 1, 1],
        }),
    );
    let ids = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    (project, ids.0, ids.1)
}
fn query(scene: &str, grid: &str) -> GridNavigationQuery {
    GridNavigationQuery {
        scene_id: scene.into(),
        grid_entity: grid.into(),
        path: GridPathRequest {
            start: [0, 1],
            end: [2, 1],
            diagonal: false,
            max_expansions: 65536,
        },
    }
}
#[test]
fn authored_grid_edits_share_validation_history_and_atomic_runtime_snapshots() {
    let (project, scene, grid) = fixture();
    let mut bus = CommandBus::new(project).unwrap();
    let mut engine = Engine::new(bus.project()).unwrap();
    let get = engine.grid_navigator();
    let req = query(&scene, &grid);
    let original = get(req.clone()).unwrap().unwrap();
    assert_eq!(original.cost, 4000);
    engine.sync(bus.project()).unwrap();
    assert_eq!(get(req.clone()).unwrap().unwrap(), original);
    let set = |value| Command::SetComponent {
        scene_id: scene.clone(),
        entity_id: grid.clone(),
        component: "NavigationGrid".into(),
        value,
    };
    for bad in [
        json!({"dimensions":[3,3],"costs":[1]}),
        json!({"dimensions":[3,3],"costs":[1,1,1,1,1001,1,1,1,1]}),
    ] {
        let before = bus.project().clone();
        assert!(
            bus.execute(
                vec![
                    Command::RenameEntity {
                        scene_id: scene.clone(),
                        entity_id: grid.clone(),
                        name: "partial".into()
                    },
                    set(bad)
                ],
                Actor::agent("grid-test", "test"),
                "reject invalid grid",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project(), &before);
        assert_eq!(bus.revision(), 0);
    }
    bus.execute(
        vec![set(json!({"dimensions":[3,3],"costs":[1,1,1,1,1,1,1,1,1]}))],
        Actor::agent("grid-test", "test"),
        "open shortcut",
        None,
    )
    .unwrap();
    engine.sync(bus.project()).unwrap();
    let short = get(req.clone()).unwrap().unwrap();
    assert_eq!(short.cost, 2000);
    assert!(short.generation > original.generation);
    assert_eq!(
        bus.project().scenes[&scene].entities[&grid]
            .provenance
            .as_ref()
            .unwrap()
            .model
            .as_deref(),
        Some("grid-test")
    );
    bus.undo().unwrap();
    engine.sync(bus.project()).unwrap();
    assert_eq!(get(req.clone()).unwrap().unwrap().cost, 4000);
    bus.redo().unwrap();
    engine.sync(bus.project()).unwrap();
    assert_eq!(get(req.clone()).unwrap().unwrap().cost, 2000);
    let before = get(req.clone()).unwrap();
    let mut invalid = bus.project().clone();
    invalid
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&grid)
        .unwrap()
        .components
        .get_mut("NavigationGrid")
        .unwrap()["costs"] = json!([]);
    assert!(engine.sync(&invalid).is_err());
    assert_eq!(get(req.clone()).unwrap(), before);
    let mut wrong = req;
    wrong.scene_id = "missing".into();
    assert!(get(wrong).is_err());
}
#[test]
fn script_grid_edits_are_visible_next_tick_and_survive_reload_and_save() {
    let (project, scene, grid) = fixture();
    let authored = project.canonical_text().unwrap();
    let req = serde_json::to_string(&query(&scene, &grid)).unwrap();
    let source = format!(
        r#"exports.default={{initialState:{{tick:0,costs:[],generations:[]}},update(api,dt,state){{
      const p=api.findGridPath({req}); state.costs.push(p.cost); state.generations.push(p.generation); state.tick++;
      if(state.tick===1)api.command({{op:'set_component',scene_id:{scene:?},entity_id:{grid:?},component:'NavigationGrid',value:{{dimensions:[3,3],costs:[1,1,1,1,1,1,1,1,1]}}}});
    }}}};"#
    );
    let mut play = PlaySession::new(&project, &source).unwrap();
    play.tick().unwrap();
    play.tick().unwrap();
    assert_eq!(play.host.state()["costs"], json!([4000, 2000]));
    assert!(
        play.host.state()["generations"][1].as_u64() > play.host.state()["generations"][0].as_u64()
    );
    play.host.hot_reload(&source).unwrap();
    play.tick().unwrap();
    assert_eq!(
        play.host.state()["generations"][1],
        play.host.state()["generations"][2]
    );
    let save = play.save_text().unwrap();
    let mut restored = PlaySession::from_save(&project, &source, &save).unwrap();
    restored.tick().unwrap();
    assert_eq!(
        restored.host.state()["costs"],
        json!([4000, 2000, 2000, 2000])
    );
    assert_eq!(
        restored.project().scenes[&scene].entities[&grid].components["NavigationGrid"]["costs"],
        json!(vec![1; 9])
    );
    assert_eq!(project.canonical_text().unwrap(), authored);
}
#[test]
fn invalid_or_over_budget_grid_queries_withhold_the_entire_failed_update() {
    let (project, scene, grid) = fixture();
    let req = serde_json::to_string(&query(&scene, &grid)).unwrap();
    for statement in [
        format!("for(let i=0;i<5;i++)api.findGridPath({req});"),
        format!("const q={req};q.path.max_expansions=1;api.findGridPath(q);"),
        format!("const q={req};q.path.end=[99,99];api.findGridPath(q);"),
        format!("const q={req};q.grid_entity='missing';api.findGridPath(q);"),
    ] {
        let source = format!(
            r#"exports.default={{initialState:{{ok:false}},update(api,dt,state){{
          state.ok=true;api.log('must not publish');api.command({{op:'set_component',scene_id:{scene:?},entity_id:{grid:?},component:'NavigationGrid',value:{{dimensions:[3,3],costs:[2,2,2,2,2,2,2,2,2]}}}});{statement}
        }}}};"#
        );
        let mut play = PlaySession::new(&project, &source).unwrap();
        assert!(play.tick().is_err());
        assert_eq!(play.host.state(), &json!({"ok":false}));
        assert_eq!(play.project(), &project);
        assert!(play.host.take_logs().is_empty());
        assert!(play.save_text().is_err());
    }
    let mut host = ScriptHost::new(&format!(
        "exports.default={{update(api){{api.findGridPath({req});}}}};"
    ))
    .unwrap();
    let mut bus = CommandBus::simulation(project).unwrap();
    assert!(host.tick(&mut bus, 1. / 60.).is_err());
}
#[test]
fn project_caps_total_grid_storage_across_scenes() {
    let mut project = Project::empty("Grid limits");
    for _ in 0..2 {
        let mut scene = Scene::new("Grid");
        let mut entity = Entity::new("Map");
        entity.components.insert(
            "NavigationGrid".into(),
            json!(NavigationGrid {
                dimensions: [256, 128],
                costs: vec![1; 32768]
            }),
        );
        scene.entities.insert(entity.id.clone(), entity);
        project.scenes.insert(scene.id.clone(), scene);
    }
    assert!(project.validate().is_ok());
    let mut extra = Entity::new("Over total");
    extra.components.insert(
        "NavigationGrid".into(),
        json!({"dimensions":[1,1],"costs":[1]}),
    );
    project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .insert(extra.id.clone(), extra);
    assert!(
        project
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("65536"))
    );
}
