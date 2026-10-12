use super::*;
use incant_runtime::{AnimationGraphControl, FrameCommands, StructuralCommand};
fn attach(p: &mut Project, sid: &str, eid: &str) -> (String, String, String) {
    let parameter = new_id();
    let first = new_id();
    let blend = new_id();
    let e = p
        .scenes
        .get_mut(sid)
        .unwrap()
        .entities
        .get_mut(eid)
        .unwrap();
    e.components.get_mut("Animation").unwrap()["seconds"] = json!(0);
    e.components.insert("AnimationGraph".into(),json!({"initial":first,"parameters":{parameter.clone():{"initial":0,"min":0,"max":1}},"states":{first.clone():{"motion":{"kind":"clip","clip":0},"looping":false},blend.clone():{"motion":{"kind":"blend1d","parameter":parameter,"cycle_seconds":2,"points":[{"threshold":0,"clip":0},{"threshold":1,"clip":1}]},"looping":true}},"transitions":{}}));
    (parameter, first, blend)
}
#[test]
fn graph_cook_uses_captured_asset_then_runs_after_all_source_and_cache_deletion() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, _) = project(root.path(), true);
    let (parameter, _, blend) = attach(&mut p, &sid, &eid);
    let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    let bytes = cook_scene_with_resources(&p, &sid, &Default::default(), &resources).unwrap();
    assert_eq!(&bytes[8..12], &scene_format(14).to_le_bytes());
    drop(resources);
    drop(p);
    root.close().unwrap();
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    let mut c = FrameCommands::default();
    for control in [
        AnimationGraphControl::SetParameter {
            parameter: stable_id(&parameter).unwrap(),
            value: 0.25,
        },
        AnimationGraphControl::SelectState(stable_id(&blend).unwrap()),
    ] {
        c.push(StructuralCommand::ControlAnimationGraph {
            entity: stable_id(&eid).unwrap(),
            control,
        })
        .unwrap();
    }
    world.apply(&mut c).unwrap();
    for _ in 0..30 {
        world.step().unwrap();
    }
    let row = world
        .animation_graph_state(stable_id(&eid).unwrap())
        .unwrap();
    assert_eq!(row.state, stable_id(&blend).unwrap());
    assert!((row.seconds - 0.5).abs() < 1e-12);
    assert_eq!(row.blend.unwrap().weight, 0.25);
    let save = world.save_bytes().unwrap();
    let mut resumed = NativeWorld::from_save_bytes(&save).unwrap();
    for _ in 0..17 {
        world.step().unwrap();
        resumed.step().unwrap();
    }
    assert_eq!(world.save_bytes().unwrap(), resumed.save_bytes().unwrap());
}
#[test]
fn graph_references_are_checked_against_the_same_captured_model_table() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, _) = project(root.path(), true);
    let (_, _, blend) = attach(&mut p, &sid, &eid);
    let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .get_mut("AnimationGraph")
        .unwrap()["states"][&blend]["motion"]["points"][1]["clip"] = json!(3);
    p.validate().unwrap();
    assert!(cook_scene_with_resources(&p, &sid, &Default::default(), &resources).is_err());
}
