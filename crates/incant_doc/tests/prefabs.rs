#[path = "support/legacy_prefab_reader.rs"]
mod legacy;
use incant_doc::*;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
fn fixture() -> (Project, Id, Id) {
    let mut p = Project::empty("Prefab");
    let node = new_id();
    let id = new_id();
    p.prefabs.insert(
        id.clone(),
        Prefab {
            id: id.clone(),
            name: "Mover".into(),
            root: node.clone(),
            provenance: None,
            nodes: BTreeMap::from([(
                node,
                PrefabNode {
                    name: "Mover".into(),
                    parent: None,
                    components: BTreeMap::from([("Transform".into(), json!(Transform::default()))]),
                },
            )]),
        },
    );
    let mut scene = Scene::new("World");
    let mut e = Entity::new("Mover");
    e.components
        .insert("Transform".into(), json!(Transform::default()));
    e.prefab = Some(PrefabLink {
        source: id,
        root: e.id.clone(),
        node: p.prefabs.values().next().unwrap().root.clone(),
        overrides: BTreeSet::new(),
    });
    let eid = e.id.clone();
    scene.entities.insert(e.id.clone(), e);
    let sid = scene.id.clone();
    p.scenes.insert(sid.clone(), scene);
    (p, sid, eid)
}
#[test]
fn legacy_v1_bytes_stay_identical_and_old_reader_refuses_new_fields() {
    let mut old = Project::empty("Legacy");
    // This fixture represents input written before engine declarations existed.
    old.engine_version = None;
    let mut scene = Scene::new("World");
    let e = Entity::new("Entity");
    scene.entities.insert(e.id.clone(), e);
    old.scenes.insert(scene.id.clone(), scene);
    let text = old.canonical_text().unwrap();
    let legacy: legacy::Project = serde_json::from_str(&text).unwrap();
    assert_eq!(
        text,
        serde_json::to_string_pretty(&serde_json::to_value(legacy).unwrap()).unwrap() + "\n"
    );
    assert_eq!(
        text,
        Project::from_text(&text).unwrap().canonical_text().unwrap()
    );
    assert!(!text.contains("prefab"));
    let (linked, _, _) = fixture();
    let mut v = serde_json::to_value(&linked).unwrap();
    // Isolate the prefab refusal from the independently tested engine declaration.
    v.as_object_mut().unwrap().remove("engine_version");
    assert!(
        serde_json::from_value::<legacy::Project>(v.clone())
            .unwrap_err()
            .to_string()
            .contains("unknown field `prefabs`")
    );
    let entity = &linked
        .scenes
        .values()
        .next()
        .unwrap()
        .entities
        .values()
        .next()
        .unwrap();
    assert!(
        serde_json::from_value::<legacy::Entity>(json!(entity))
            .unwrap_err()
            .to_string()
            .contains("unknown field `prefab`")
    );
    let mut future = serde_json::to_value(old).unwrap();
    future["schema_version"] = json!(2);
    assert!(matches!(
        Project::from_text(&future.to_string()),
        Err(DocumentError::Version(2))
    ));
}
#[test]
fn linked_text_and_crdt_reopen_are_exact() {
    let (p, _, _) = fixture();
    let text = p.canonical_text().unwrap();
    assert_eq!(Project::from_text(&text).unwrap(), p);
    let d = CollaborativeDocument::new(&p).unwrap();
    assert_eq!(
        CollaborativeDocument::from_export(&d.export().unwrap())
            .unwrap()
            .project()
            .unwrap(),
        p
    );
    let schemas = schema_registry();
    assert!(schemas.contains_key("Prefab"));
    assert!(schemas.contains_key("PrefabLink"));
}
#[test]
fn hostile_link_and_unrecorded_component_divergence_refuse() {
    let (p, s, e) = fixture();
    p.validate().unwrap();
    for case in 0..8 {
        let mut bad = p.clone();
        let entity = bad
            .scenes
            .get_mut(&s)
            .unwrap()
            .entities
            .get_mut(&e)
            .unwrap();
        match case {
            0 => entity.prefab.as_mut().unwrap().root = new_id(),
            1 => entity.prefab.as_mut().unwrap().node = new_id(),
            2 => entity.prefab.as_mut().unwrap().source = new_id(),
            3 => entity.name = "Forged".into(),
            4 => {
                entity.components.get_mut("Transform").unwrap()["translation"] = json!([2, 0, 0]);
            }
            5 => {
                entity
                    .prefab
                    .as_mut()
                    .unwrap()
                    .overrides
                    .insert("Prefab".into());
            }
            6 => {
                let mut child = Entity::new("Foreign");
                child.parent = Some(e.clone());
                bad.scenes
                    .get_mut(&s)
                    .unwrap()
                    .entities
                    .insert(child.id.clone(), child);
            }
            _ => {
                bad.prefabs
                    .values_mut()
                    .next()
                    .unwrap()
                    .nodes
                    .values_mut()
                    .next()
                    .unwrap()
                    .parent = Some(new_id());
            }
        }
        assert!(bad.validate().is_err(), "case {case}");
    }
}
#[test]
fn malformed_unused_templates_and_deep_payloads_refuse() {
    let (mut p, _, _) = fixture();
    p.scenes.clear();
    p.prefabs
        .values_mut()
        .next()
        .unwrap()
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .components
        .insert("Velocity".into(), json!({"linear":[1,2]}));
    assert!(p.validate().is_err());
    let mut v = json!(0);
    for _ in 0..65 {
        v = json!([v]);
    }
    p.prefabs
        .values_mut()
        .next()
        .unwrap()
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .components
        .insert("Velocity".into(), v);
    assert!(p.diagnostics().iter().any(|d| d.message.contains("depth")));
}

#[test]
fn all_typed_entity_references_remap_without_rewriting_asset_or_literal_ids() {
    let a = new_id();
    let b = new_id();
    let x = new_id();
    let y = new_id();
    let mapping = BTreeMap::from([(a.clone(), x.clone()), (b.clone(), y.clone())]);
    let source = BTreeMap::from([
        ("AudioBus".into(), json!({"parent":a,"gain_db":0})),
        (
            "AudioSource".into(),
            json!(AudioSource {
                clip: a.clone(),
                bus: Some(b.clone()),
                gain_db: 0.0,
                pan: 0.0,
                rate: 1.0,
                start_seconds: 0.0,
                playing: false,
                looping: false,
                streaming: false,
                spatial: None,
            }),
        ),
        (
            "NavigationMesh".into(),
            json!(NavigationMesh {
                settings: Default::default(),
                links: vec![],
                sources: vec![
                    NavigationSource {
                        entity: a.clone(),
                        geometry: NavigationSourceKind::Collider,
                    },
                    NavigationSource {
                        entity: b.clone(),
                        geometry: NavigationSourceKind::Mesh
                    }
                ],
            }),
        ),
        ("Script".into(), json!({"source":a,"props":{"entity":b}})),
        ("MeshRenderer".into(), json!({"mesh":a,"materials":[b]})),
    ]);
    let resolved = prefab::remap_components(&source, &mapping).unwrap();
    assert_eq!(resolved["AudioBus"]["parent"], x);
    assert_eq!(resolved["AudioSource"]["bus"], y);
    assert_eq!(resolved["AudioSource"]["clip"], a);
    assert_eq!(resolved["NavigationMesh"]["sources"][0]["entity"], x);
    assert_eq!(resolved["NavigationMesh"]["sources"][1]["entity"], y);
    assert_eq!(resolved["Script"], source["Script"]);
    assert_eq!(resolved["MeshRenderer"], source["MeshRenderer"]);
    for kind in ["AudioBus", "AudioSource", "NavigationMesh"] {
        let component = BTreeMap::from([(kind.into(), source[kind].clone())]);
        assert!(
            prefab::remap_components(&component, &BTreeMap::new()).is_err(),
            "{kind}"
        );
    }
}
