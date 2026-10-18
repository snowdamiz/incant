use incant_doc::{Asset, Project, RustModuleSource, Scene, new_id};
use serde_json::json;

fn module(path: &str) -> RustModuleSource {
    RustModuleSource {
        id: new_id(),
        name: "Movement".into(),
        path: path.into(),
    }
}

#[test]
fn old_documents_keep_their_canonical_bytes_and_new_registration_roundtrips() {
    let text = serde_json::to_string_pretty(&json!({
        "id":"01ARZ3NDEKTSV4RRFFQ69G5FAV", "schema_version":1,
        "name":"Legacy", "scenes":{}, "assets":{}, "scripts":{},
        "settings":{"tick_rate":60}, "memory":{}
    }))
    .unwrap()
        + "\n";
    let mut project = Project::from_text(&text).unwrap();
    assert!(project.module_sources.is_empty());
    assert_eq!(project.canonical_text().unwrap(), text);
    let source = module("movement/src/lib.rs");
    project.module_sources.insert(source.id.clone(), source);
    let actual = project.canonical_text().unwrap();
    assert_eq!(Project::from_text(&actual).unwrap(), project);
    assert!(project.assets.is_empty());
    assert!(project.scenes.is_empty());
    let schema = json!(schemars::schema_for!(Project));
    assert!(
        !schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("module_sources"))
    );
    let mut unknown = json!(project);
    let id = project.module_sources.keys().next().unwrap();
    unknown["module_sources"][id]["compiled_asset"] = json!(new_id());
    assert!(Project::from_text(&unknown.to_string()).is_err());
}

#[test]
fn registered_ids_are_global_and_path_aliases_refuse() {
    let mut project = Project::empty("Identity");
    let source = module("movement/src/lib.rs");
    project
        .module_sources
        .insert(source.id.clone(), source.clone());
    project.validate().unwrap();
    let mut invalid = project.clone();
    invalid.module_sources.get_mut(&source.id).unwrap().id = new_id();
    assert!(invalid.validate().is_err());
    let mut scene = Scene::new("Collision");
    scene.id = source.id.clone();
    invalid = project.clone();
    invalid.scenes.insert(scene.id.clone(), scene);
    assert!(
        invalid
            .diagnostics()
            .iter()
            .any(|d| d.message == "ID must be globally unique")
    );
    invalid = project.clone();
    let alias = module("MOVEMENT/src/LIB.rs");
    invalid.module_sources.insert(alias.id.clone(), alias);
    assert!(invalid.validate().is_err());
    let asset = Asset {
        id: new_id(),
        name: "Authored input".into(),
        path: source.path.clone(),
        kind: "wasm".into(),
        sha256: "ab".repeat(32),
        import_settings: None,
    };
    project.assets.insert(asset.id.clone(), asset);
    assert!(project.validate().is_err());
}

#[test]
fn portable_source_only_grammar_refuses_unsafe_or_unbuildable_registration() {
    let mut project = Project::empty("Paths");
    let source = module("movement/main.rs");
    project
        .module_sources
        .insert(source.id.clone(), source.clone());
    for path in [
        "main.rs",
        "../main.rs",
        "a//main.rs",
        "/a/main.rs",
        "a\\main.rs",
        "a/main.RS",
        "a/main.ts",
        "a/build.rs",
        "a/BUILD.RS/main.rs",
        "a/Cargo.toml",
        "-a/main.rs",
        "a/-main.rs",
        "a/é.rs",
        "a/.hidden.rs",
        "a/CON.rs",
        "nul/main.rs",
        "a/COM1.rs",
        "a./main.rs",
        "a /main.rs",
        "a/main.rs:stream",
        "a/main?.rs",
    ] {
        project.module_sources.get_mut(&source.id).unwrap().path = path.into();
        assert!(project.validate().is_err(), "{path}");
    }
    for path in [
        format!("a/{}.rs", "x".repeat(126)),
        "a/".repeat(8) + "main.rs",
    ] {
        project.module_sources.get_mut(&source.id).unwrap().path = path;
        assert!(project.validate().is_err());
    }
    for path in ["_module/src/main.rs".into(), "a/".repeat(7) + "main.rs"] {
        project.module_sources.get_mut(&source.id).unwrap().path = path;
        project.validate().unwrap();
    }
    for name in [" ".into(), "x".repeat(257)] {
        project.module_sources.get_mut(&source.id).unwrap().name = name;
        assert!(project.validate().is_err());
    }
}

#[cfg(feature = "crdt")]
#[test]
fn crdt_materializes_optional_registry_and_merges_independent_metadata() {
    use incant_doc::CollaborativeDocument;
    let initial = Project::empty("CRDT source identities");
    let mut left = CollaborativeDocument::new(&initial).unwrap();
    let mut right = left.fork();
    let first = module("movement/main.rs");
    let second = module("camera/main.rs");
    let mut left_project = initial.clone();
    left_project
        .module_sources
        .insert(first.id.clone(), first.clone());
    left.replace(&left_project).unwrap();
    let mut right_project = initial.clone();
    right_project
        .module_sources
        .insert(second.id.clone(), second.clone());
    right.replace(&right_project).unwrap();
    left.merge(&right.export().unwrap()).unwrap();
    let merged = CollaborativeDocument::from_export(&left.export().unwrap())
        .unwrap()
        .project()
        .unwrap();
    assert_eq!(merged.module_sources.len(), 2);
    assert_eq!(merged.module_sources[&first.id], first);
    assert_eq!(merged.module_sources[&second.id], second);
    assert!(merged.assets.is_empty());
    assert!(merged.scripts.is_empty());
    // Old empty-field input remains absent after an ordinary CRDT roundtrip.
    let empty = CollaborativeDocument::new(&initial)
        .unwrap()
        .project()
        .unwrap();
    assert_eq!(
        empty.canonical_text().unwrap(),
        initial.canonical_text().unwrap()
    );
}
