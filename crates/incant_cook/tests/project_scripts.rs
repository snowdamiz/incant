#![cfg(all(feature = "scene", feature = "script"))]
use incant_cook::{CookError, cook_scene, prepare_script_scene, stable_id};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use incant_script::NativeSession;
use serde_json::json;

fn fixture() -> (Project, String, String, String) {
    let mut project = Project::empty("Attached gameplay");
    project.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Mover");
    let source = new_id();
    entity.components.insert(
        "Script".into(),
        json!({"source":source,"props":{"speed":2,"nested":{"__proto__":"data"}}}),
    );
    let eid = entity.id.clone();
    let sid = scene.id.clone();
    scene.entities.insert(eid.clone(), entity);
    project.scenes.insert(sid.clone(), scene);
    project.scripts.insert(
        source.clone(),
        ScriptSource {
            id: source.clone(),
            name: "Mover".into(),
            path: "scripts/move.ts".into(),
        },
    );
    (project, sid, eid, source)
}

#[test]
fn program_cook_retains_checked_attachments_and_plain_cook_still_refuses_them() {
    let (project, sid, eid, source) = fixture();
    let before = project.canonical_text().unwrap();
    assert!(
        matches!(cook_scene(&project,&sid),Err(CookError::UnsupportedComponent{component,..}) if component=="Script")
    );
    let prepared = prepare_script_scene(&project, &sid).unwrap();
    assert_eq!(
        prepared.source_catalog(),
        &json!([{"id":source,"path":"scripts/move.ts"}])
    );
    let code = format!(
        "exports.default={{sources:[{{id:{source:?},behavior:{{update({{chunks,instances,dt}}){{for(const chunk of chunks)for(let row=0;row<chunk.ids.length;row++)if(chunk.ids[row]===instances[0].entity)chunk.transforms[row*10]+=instances[0].props.speed*dt;}}}}}}]}};"
    );
    let (scene, script) = prepared.compile(&code).unwrap().into_parts();
    assert_eq!(project.canonical_text().unwrap(), before);
    drop(project);
    let mut session = NativeSession::new(scene, script, Default::default()).unwrap();
    for _ in 0..60 {
        session.tick(&[]).unwrap();
    }
    assert!(
        (session
            .world()
            .unwrap()
            .inspect(stable_id(&eid).unwrap())
            .unwrap()
            .transform
            .translation[0]
            - 2.)
            .abs()
            < 1e-12
    );
}

#[test]
fn invalid_sources_unsupported_components_and_aliases_refuse_the_whole_cook() {
    let (original, sid, eid, source) = fixture();
    let mut invalid = original.clone();
    invalid.scripts.clear();
    assert!(prepare_script_scene(&invalid, &sid).is_err());
    for path in [
        "scripts/move.js",
        "scripts/move.d.ts",
        "../outside.ts",
        "scripts/../move.ts",
        "/absolute.ts",
    ] {
        let mut invalid = original.clone();
        invalid.scripts.get_mut(&source).unwrap().path = path.into();
        assert!(prepare_script_scene(&invalid, &sid).is_err(), "{path}");
    }
    let mut invalid = original.clone();
    invalid
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert("UnknownCapability".into(), json!({}));
    assert!(prepare_script_scene(&invalid, &sid).is_err());
    let mut invalid = original.clone();
    let duplicate = new_id();
    invalid.scripts.insert(
        duplicate.clone(),
        ScriptSource {
            id: duplicate.clone(),
            name: "Alias".into(),
            path: "scripts/move.ts".into(),
        },
    );
    let mut entity = Entity::new("Alias");
    entity
        .components
        .insert("Script".into(), json!({"source":duplicate,"props":{}}));
    invalid
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .insert(entity.id.clone(), entity);
    assert!(prepare_script_scene(&invalid, &sid).is_err());
    let mut invalid = original.clone();
    invalid
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .remove("Script");
    assert!(prepare_script_scene(&invalid, &sid).is_err());
}
