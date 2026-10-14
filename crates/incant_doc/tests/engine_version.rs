use incant_doc::{EngineDeclaration, EngineRequirement, Project};
use serde_json::json;

fn legacy() -> String {
    serde_json::to_string_pretty(
        &json!({"id":"01ARZ3NDEKTSV4RRFFQ69G5FAV", "schema_version":1,
        "name":"Legacy", "scenes":{}, "assets":{}, "scripts":{},
        "settings":{"tick_rate":60}, "memory":{}}),
    )
    .unwrap()
        + "\n"
}

#[test]
fn missing_legacy_declaration_preserves_bytes_but_new_projects_have_exact_pin() {
    let text = legacy();
    let project = Project::from_text(&text).unwrap();
    assert_eq!(project.engine_version, None);
    assert_eq!(project.canonical_text().unwrap(), text);
    assert_eq!(
        project.engine_compatibility().unwrap().declaration,
        EngineDeclaration::LegacyUnpinned
    );
    let fresh = Project::empty("New");
    assert_eq!(fresh.engine_version.as_ref().unwrap().as_str(), "=0.0.1");
    let current = fresh.canonical_text().unwrap();
    assert_eq!(Project::from_text(&current).unwrap(), fresh);
    assert_eq!(
        fresh.engine_compatibility().unwrap().declaration,
        EngineDeclaration::Compatible
    );
}

#[test]
fn requirements_are_bounded_typed_and_apply_to_direct_validation_too() {
    let mut project = Project::empty("Requirements");
    for valid in ["=0.0.1", ">=0.0.1, <0.1.0", "^0.0.1", "*", "=0.0.1+build-a"] {
        project.engine_version = Some(EngineRequirement::parse(valid).unwrap());
        let text = project.canonical_text().unwrap();
        assert_eq!(
            Project::from_text(&text)
                .unwrap()
                .engine_version
                .as_ref()
                .unwrap()
                .as_str(),
            valid
        );
    }
    for unsupported in ["=0.0.2", "<0.0.1", "=0.0.1-alpha.1", ">=99.0.0"] {
        project.engine_version = Some(EngineRequirement::parse(unsupported).unwrap());
        assert!(project.validate().is_err());
        assert!(project.validated().is_err());
        assert!(Project::from_text(&serde_json::to_string(&project).unwrap()).is_err());
    }
    let sixteen = std::iter::repeat_n(">=0", 16).collect::<Vec<_>>().join(",");
    assert!(EngineRequirement::parse(sixteen.clone()).is_ok());
    assert!(EngineRequirement::parse(format!("{sixteen},>=0")).is_err());
    for invalid in ["", " ", "=", "0.0.1 || 0.0.2", "01.0.0", "garbage"] {
        assert!(EngineRequirement::parse(invalid).is_err(), "{invalid}");
    }
    assert!(EngineRequirement::parse("*".to_owned() + &" ".repeat(256)).is_err());
    for invalid in [json!(1), json!(true), json!([]), json!({"version":"0.0.1"})] {
        let mut value = json!(Project::empty("Malformed"));
        value["engine_version"] = invalid;
        assert!(Project::from_text(&value.to_string()).is_err());
    }
}

#[test]
fn format_versions_stay_closed_no_automatic_repin_or_format_conversion() {
    for version in [0, 2, u32::MAX as u64 + 1] {
        let mut value: serde_json::Value = serde_json::from_str(&legacy()).unwrap();
        value["schema_version"] = json!(version);
        assert!(Project::from_text(&value.to_string()).is_err());
    }
}

// Project's main269 fields, before this additive declaration. Keeping a strict
// concrete reader demonstrates that an old reader fails closed on new data.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(
    dead_code,
    reason = "frozen old reader shape is used only for deserialization refusal"
)]
struct OldProject {
    id: incant_doc::Id,
    schema_version: u32,
    name: String,
    scenes: std::collections::BTreeMap<incant_doc::Id, incant_doc::Scene>,
    assets: std::collections::BTreeMap<incant_doc::Id, incant_doc::Asset>,
    scripts: std::collections::BTreeMap<incant_doc::Id, incant_doc::ScriptSource>,
    #[serde(default)]
    string_tables: std::collections::BTreeMap<incant_doc::Id, incant_doc::StringTable>,
    settings: incant_doc::ProjectSettings,
    memory: std::collections::BTreeMap<String, String>,
}

#[test]
fn older_reader_accepts_legacy_and_refuses_new_field_and_schema_exposes_bound() {
    assert!(serde_json::from_str::<OldProject>(&legacy()).is_ok());
    let err = serde_json::from_str::<OldProject>(&Project::empty("New").canonical_text().unwrap())
        .err()
        .unwrap();
    assert!(err.to_string().contains("unknown field `engine_version`"));
    let schema = json!(schemars::schema_for!(Project));
    assert!(
        !schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("engine_version"))
    );
    assert_eq!(schema["$defs"]["EngineRequirement"]["maxLength"], 256);
}
