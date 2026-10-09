use incant_doc::*;
use serde_json::json;

fn sample() -> Project {
    let mut p = Project::empty("Test");
    let mut scene = Scene::new("World");
    let mut entity = Entity::new("Player");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    scene.entities.insert(entity.id.clone(), entity);
    p.scenes.insert(scene.id.clone(), scene);
    p
}
#[test]
fn deterministic_roundtrip() {
    let p = sample();
    let text = p.canonical_text().unwrap();
    assert_eq!(
        Project::from_text(&text).unwrap().canonical_text().unwrap(),
        text
    );
}
#[test]
fn rejects_unknown_versions_fields_and_cycles() {
    let mut p = sample();
    let scene = p.scenes.values_mut().next().unwrap();
    let entity = scene.entities.values_mut().next().unwrap();
    entity.parent = Some(entity.id.clone());
    assert!(p.validate().is_err());
    let mut v = json!(sample());
    v["schema_version"] = json!(2);
    assert!(matches!(
        Project::from_text(&v.to_string()),
        Err(DocumentError::Version(2))
    ));
    v["schema_version"] = json!(1);
    v["surprise"] = json!(true);
    assert!(Project::from_text(&v.to_string()).is_err());
}
#[test]
fn rejects_cross_platform_path_escapes() {
    for path in ["../x", "/tmp/x", "C:/x", "..\\x", "x//y", "x/../y", "x\0y"] {
        assert!(!safe_relative_path(path), "{path}");
    }
    assert!(safe_relative_path("assets/stone.glb"));
}
#[test]
fn concurrent_fields_survive_and_converge() {
    let p = sample();
    let mut a = CollaborativeDocument::new(&p).unwrap();
    let mut b = a.fork();
    let mut pa = a.project().unwrap();
    let mut pb = b.project().unwrap();
    pa.scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
        .next()
        .unwrap()
        .name = "A".into();
    pb.scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
        .next()
        .unwrap()
        .components
        .get_mut("Transform")
        .unwrap()["translation"] = json!([3., 2., 1.]);
    a.replace(&pa).unwrap();
    b.replace(&pb).unwrap();
    let ea = a.export().unwrap();
    let eb = b.export().unwrap();
    a.merge(&eb).unwrap();
    b.merge(&ea).unwrap();
    assert_eq!(a.project().unwrap(), b.project().unwrap());
    let out = a.project().unwrap();
    let e = out
        .scenes
        .values()
        .next()
        .unwrap()
        .entities
        .values()
        .next()
        .unwrap();
    assert_eq!(e.name, "A");
    assert_eq!(
        e.components["Transform"]["translation"],
        json!([3., 2., 1.])
    );
}
#[test]
fn concurrent_creation_of_component_merges_fields() {
    let p = sample();
    let mut a = CollaborativeDocument::new(&p).unwrap();
    let mut b = a.fork();
    let mut pa = p.clone();
    let mut pb = p.clone();
    pa.memory.insert("a".into(), "one".into());
    pb.memory.insert("b".into(), "two".into());
    a.replace(&pa).unwrap();
    b.replace(&pb).unwrap();
    a.merge(&b.export().unwrap()).unwrap();
    assert_eq!(a.project().unwrap().memory.len(), 2);
}
#[test]
fn bad_merge_does_not_corrupt_live_document() {
    let p = sample();
    let mut a = CollaborativeDocument::new(&p).unwrap();
    assert!(a.merge(b"malformed").is_err());
    assert_eq!(a.project().unwrap(), p);
}
#[test]
fn semantic_invalid_merge_is_atomic() {
    let mut p = sample();
    let scene = p.scenes.values_mut().next().unwrap();
    let a = scene.entities.keys().next().unwrap().clone();
    let b = Entity::new("Child");
    let bid = b.id.clone();
    scene.entities.insert(b.id.clone(), b);
    let mut left = CollaborativeDocument::new(&p).unwrap();
    let mut right = left.fork();
    let mut pl = p.clone();
    let mut pr = p.clone();
    pl.scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&a)
        .unwrap()
        .parent = Some(bid.clone());
    pr.scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .get_mut(&bid)
        .unwrap()
        .parent = Some(a);
    left.replace(&pl).unwrap();
    right.replace(&pr).unwrap();
    assert!(left.merge(&right.export().unwrap()).is_err());
    assert_eq!(left.project().unwrap(), pl);
}
