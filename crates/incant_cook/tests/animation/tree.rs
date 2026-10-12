use super::*;

#[test]
fn authored_nested_grid_cooks_in_postorder_and_survives_source_deletion() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, _) = project(root.path(), true);
    let parameter = new_id();
    let other = new_id();
    let state = new_id();
    let a = new_id();
    let b = new_id();
    let c = new_id();
    let rest = new_id();
    let grid = new_id();
    let tree_root = new_id();
    let e = p
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap();
    e.components.insert(
        "Animation".into(),
        json!({"clip":null,"seconds":0,"speed":1,"playing":true,"looping":true}),
    );
    e.components.insert("AnimationGraph".into(),json!({"initial":state,"parameters":{
        parameter.clone():{"initial":0.25,"min":0,"max":1},other.clone():{"initial":0.5,"min":0,"max":1}},
        "states":{state.clone():{"looping":true,"motion":{"kind":"tree","cycle_seconds":2,"tree":{"root":tree_root,"nodes":{
            a.clone():{"kind":"clip","clip":0},b.clone():{"kind":"clip","clip":1},c.clone():{"kind":"clip","clip":2},rest.clone():{"kind":"clip","clip":null},
            grid.clone():{"kind":"blend2d","parameters":[parameter,other],"x":[0,1],"y":[0,1],"children":[a,b,c,rest.clone()]},
            tree_root:{"kind":"blend1d","parameter":parameter,"points":[{"threshold":0,"node":grid},{"threshold":1,"node":rest}]}
        }}}}},"transitions":{}}));
    p.validate().unwrap();
    let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    let bytes = cook_scene_with_resources(&p, &sid, &Default::default(), &resources).unwrap();
    assert_eq!(&bytes[8..12], &15u32.to_le_bytes());
    // Authored clip bounds are checked against the captured model, including
    // leaves that have zero weight at this parameter value.
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .get_mut("AnimationGraph")
        .unwrap()["states"][&state]["motion"]["tree"]["nodes"][&a]["clip"] = json!(3);
    p.validate().unwrap();
    assert!(cook_scene_with_resources(&p, &sid, &Default::default(), &resources).is_err());
    drop(resources);
    drop(p);
    root.close().unwrap();
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..30 {
        world.step().unwrap();
    }
    let pose = world
        .animation_graph_state(stable_id(&eid).unwrap())
        .unwrap()
        .pose
        .unwrap();
    assert_eq!(pose.samples().len(), 4);
    for entry in pose.samples() {
        let expected = match entry.sample.clip {
            Some(0) => 0.28125,
            Some(1) => 0.09375,
            Some(2) => 0.28125,
            None => 0.34375,
            _ => panic!("extra leaf"),
        };
        assert_eq!(entry.weight, expected);
        assert!(
            (entry.sample.seconds
                - if entry.sample.clip.is_some() {
                    0.5
                } else {
                    0.0
                })
            .abs()
                < 1e-12
        );
    }
    let mut resumed = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    for _ in 0..37 {
        world.step().unwrap();
        resumed.step().unwrap();
    }
    assert_eq!(world.save_bytes().unwrap(), resumed.save_bytes().unwrap());
}
