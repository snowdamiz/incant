use incant_doc::{PhysicsMode, Project};
use serde_json::json;

#[test]
fn absent_mode_preserves_legacy_canonical_bytes_and_fast_round_trips() {
    let old = json!({"id":"01ARZ3NDEKTSV4RRFFQ69G5FAV", "schema_version":1,
        "name":"Legacy", "scenes":{}, "assets":{}, "scripts":{},
        "settings":{"tick_rate":60}, "memory":{}});
    let bytes = serde_json::to_string_pretty(&old).unwrap() + "\n";
    let mut project = Project::from_text(&bytes).unwrap();
    assert_eq!(project.settings.physics_mode, PhysicsMode::Deterministic);
    assert_eq!(project.canonical_text().unwrap(), bytes);
    project.settings.physics_mode = PhysicsMode::Fast;
    let fast = project.canonical_text().unwrap();
    assert_eq!(Project::from_text(&fast).unwrap(), project);
    assert_eq!(json!(project)["settings"]["physics_mode"], "fast");
    project.settings.physics_mode = PhysicsMode::Deterministic;
    assert_eq!(project.canonical_text().unwrap(), bytes);
}

#[test]
fn document_and_published_schema_admit_only_two_authoring_modes() {
    let schema = json!(schemars::schema_for!(Project));
    assert_eq!(
        schema["$defs"]["PhysicsMode"]["enum"],
        json!(["fast", "deterministic"])
    );
    assert_eq!(
        schema["$defs"]["ProjectSettings"]["properties"]["physics_mode"]["default"],
        "deterministic"
    );
    for invalid in [
        json!(null),
        json!(true),
        json!(0),
        json!("Fast"),
        json!("fast_simd8"),
        json!({}),
    ] {
        let mut value = json!(Project::empty("Reject unknown mode"));
        value["settings"]["physics_mode"] = invalid;
        assert!(Project::from_text(&value.to_string()).is_err());
    }
}
