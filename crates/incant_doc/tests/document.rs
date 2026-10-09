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
fn environments_validate_bindings_limits_and_global_uniqueness() {
    let mut project = sample();
    let id = new_id();
    project.assets.insert(
        id.clone(),
        Asset {
            id: id.clone(),
            name: "Studio".into(),
            path: "studio.exr".into(),
            kind: "texture".into(),
            sha256: "ab".repeat(32),
            import_settings: Some(AssetImportSettings::Texture {
                usage: TextureUsage::Linear,
            }),
        },
    );
    let sid = project.scenes.keys().next().unwrap().clone();
    let eid = project.scenes[&sid].entities.keys().next().unwrap().clone();
    let valid = json!(EnvironmentLight {
        texture: id.clone(),
        intensity: 1.,
        rotation_degrees: 180.
    });
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert("EnvironmentLight".into(), valid.clone());
    let serialized = project.canonical_text().unwrap();
    assert_eq!(Project::from_text(&serialized).unwrap(), project);
    for (field, value) in [
        ("texture", json!(new_id())),
        ("intensity", json!(-0.1)),
        ("intensity", json!(100.1)),
        ("rotation_degrees", json!(361)),
        ("unknown", json!(0)),
    ] {
        let mut invalid = project.clone();
        invalid
            .scenes
            .get_mut(&sid)
            .unwrap()
            .entities
            .get_mut(&eid)
            .unwrap()
            .components
            .get_mut("EnvironmentLight")
            .unwrap()[field] = value;
        assert!(invalid.validate().is_err(), "{field}");
    }
    for usage in [
        None,
        Some(AssetImportSettings::Texture {
            usage: TextureUsage::Color,
        }),
        Some(AssetImportSettings::Texture {
            usage: TextureUsage::Linear,
        }),
    ] {
        project.assets.get_mut(&id).unwrap().import_settings = usage;
        project.validate().unwrap();
    }
    project.assets.get_mut(&id).unwrap().import_settings = Some(AssetImportSettings::Texture {
        usage: TextureUsage::Normal,
    });
    assert!(project.validate().is_err());
    project.assets.get_mut(&id).unwrap().import_settings = None;
    project.assets.get_mut(&id).unwrap().kind = "model".into();
    assert!(project.validate().is_err());
    project.assets.get_mut(&id).unwrap().kind = "texture".into();
    let mut second = Scene::new("Other scene");
    let mut entity = Entity::new("Other environment");
    entity.components.insert("EnvironmentLight".into(), valid);
    second.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(second.id.clone(), second);
    assert!(
        project
            .diagnostics()
            .iter()
            .any(|d| d.message.contains("only one global"))
    );
}
#[test]
fn mesh_bindings_require_the_correct_asset_kinds() {
    let mut project = sample();
    let model = Asset {
        id: new_id(),
        name: "Model".into(),
        path: "model.glb".into(),
        kind: "model".into(),
        sha256: "ab".repeat(32),
        import_settings: None,
    };
    let material = Asset {
        id: new_id(),
        name: "Material".into(),
        path: "material.json".into(),
        kind: "material".into(),
        sha256: "cd".repeat(32),
        import_settings: None,
    };
    project.assets.insert(model.id.clone(), model.clone());
    project.assets.insert(material.id.clone(), material.clone());
    let entity = project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
        .next()
        .unwrap();
    entity.components.insert(
        "MeshRenderer".into(),
        json!(MeshRenderer {
            mesh: model.id.clone(),
            materials: vec![material.id.clone()],
            cast_shadows: true
        }),
    );
    project.validate().unwrap();
    project.assets.get_mut(&model.id).unwrap().kind = "texture".into();
    assert!(
        project
            .diagnostics()
            .iter()
            .any(|error| error.message == "mesh reference must identify a model asset")
    );
    project.assets.get_mut(&model.id).unwrap().kind = "model".into();
    project.assets.get_mut(&material.id).unwrap().kind = "texture".into();
    assert!(
        project
            .diagnostics()
            .iter()
            .any(|error| error.message == "material reference must identify a material asset")
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
