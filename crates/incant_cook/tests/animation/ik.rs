use super::*;
use incant_runtime::{AnimationIkSample, CookedScene};

fn attach(p: &mut Project, sid: &str, eid: &str, constraint: &str) {
    p.scenes.get_mut(sid).unwrap().entities.get_mut(eid).unwrap().components.insert("AnimationIk".into(),json!({"constraints":[{"id":constraint,"chain":[0,1,2],"target":[11,1,0],"pole":[10,1,0],"weight":1}]}));
}
#[test]
fn actual_cache_seals_topology_and_source_deleted_binary_keeps_constraint_order() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, _) = project(root.path(), true);
    let constraint = new_id();
    attach(&mut p, &sid, &eid, &constraint);
    let resources = load_project_animation_resources(&p, &sid, root.path()).unwrap();
    let bytes = cook_scene_with_resources(&p, &sid, &Default::default(), &resources).unwrap();
    assert_eq!(&bytes[8..12], &16u32.to_le_bytes());
    let before = p.canonical_text().unwrap();
    let parsed = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(
        parsed.spatial()[0].ik.as_ref().unwrap().initial[0],
        AnimationIkSample {
            id: stable_id(&constraint).unwrap(),
            chain: [0, 1, 2],
            target: [11., 1., 0.],
            pole: [10., 1., 0.],
            weight: 1.
        }
    );
    assert_eq!(p.canonical_text().unwrap(), before);
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .get_mut("AnimationIk")
        .unwrap()["constraints"][0]["chain"] = json!([0, 2, 1]);
    assert!(cook_scene_with_resources(&p, &sid, &Default::default(), &resources).is_err());
    assert!(load_project_animation_resources(&p, &sid, root.path()).is_err());
    drop(p);
    drop(resources);
    root.close().unwrap();
    let world = NativeWorld::from_bytes(&bytes).unwrap();
    assert_eq!(
        world.ik_state(stable_id(&eid).unwrap()).unwrap()[0].target,
        [11., 1., 0.]
    );
}
#[test]
fn authoring_requires_animation_and_rejects_duplicate_or_unbounded_constraints() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, sid, eid, _) = project(root.path(), true);
    let constraint = new_id();
    attach(&mut p, &sid, &eid, &constraint);
    p.validate().unwrap();
    let e = p
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap();
    let animation = e.components.remove("Animation").unwrap();
    assert!(p.validate().is_err());
    p.scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert("Animation".into(), animation);
    let ik = &mut p
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .get_mut("AnimationIk")
        .unwrap();
    let c = ik["constraints"][0].clone();
    ik["constraints"] = json!([c.clone(), c]);
    assert!(p.validate().is_err());
}
