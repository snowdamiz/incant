use incant_doc::{
    AgentBudgetSettings, DEFAULT_AGENT_TOKEN_CAP, DocumentError, MAX_AGENT_TOKEN_CAP, Project,
};
use serde_json::{Value, json};

fn legacy_project() -> Value {
    json!({
        "id": "01ARZ3NDEKTSV4RRFFQ69G5FAV", "schema_version": 1, "name": "Legacy",
        "scenes": {}, "assets": {}, "scripts": {}, "settings": {"tick_rate": 60}, "memory": {}
    })
}

#[test]
fn legacy_projects_default_without_changing_canonical_bytes() {
    let text = serde_json::to_string_pretty(&legacy_project()).unwrap() + "\n";
    let project = Project::from_text(&text).unwrap();
    assert_eq!(
        project.settings.agent_budget.max_tokens,
        DEFAULT_AGENT_TOKEN_CAP
    );
    assert_eq!(project.canonical_text().unwrap(), text);
    assert_eq!(
        json!(Project::empty("New"))["settings"],
        json!({"tick_rate": 60})
    );
    assert_eq!(AgentBudgetSettings::default().max_tokens, 1_000_000);
}

#[test]
fn disabled_and_largest_exact_caps_round_trip() {
    for max_tokens in [0, 1, MAX_AGENT_TOKEN_CAP] {
        let mut value = legacy_project();
        value["settings"]["agent_budget"] = json!({"max_tokens": max_tokens});
        let project = Project::from_text(&value.to_string()).unwrap();
        assert_eq!(project.settings.agent_budget.max_tokens, max_tokens);
        let canonical = project.canonical_text().unwrap();
        assert_eq!(Project::from_text(&canonical).unwrap(), project);
        assert_eq!(
            json!(project)["settings"]["agent_budget"],
            json!({"max_tokens": max_tokens})
        );
    }
}

#[test]
fn explicit_settings_require_a_bounded_unsigned_integer_and_no_ledger() {
    for settings in [
        json!({}),
        json!({"max_tokens": -1}),
        json!({"max_tokens": 1.5}),
        json!({"max_tokens": "100"}),
        json!({"max_tokens": true}),
        json!({"max_tokens": null}),
        json!({"max_tokens": MAX_AGENT_TOKEN_CAP + 1}),
        json!({"max_tokens": u64::MAX}),
        json!({"max_tokens": 100, "consumed_tokens": 0}),
        Value::Null,
    ] {
        let mut value = legacy_project();
        value["settings"]["agent_budget"] = settings.clone();
        assert!(
            Project::from_text(&value.to_string()).is_err(),
            "accepted {settings}"
        );
    }
    let mut project = Project::empty("Invalid typed cap");
    project.settings.agent_budget.max_tokens = MAX_AGENT_TOKEN_CAP + 1;
    let DocumentError::Validation(errors) = project.validate().unwrap_err() else {
        panic!("expected semantic validation error");
    };
    assert!(
        errors
            .iter()
            .any(|error| error.path == "/settings/agent_budget/max_tokens")
    );
    assert!(project.canonical_text().is_err());
    assert!(project.validated().is_err());
}

#[test]
fn published_schema_expresses_the_default_and_exact_integer_limit() {
    let schema = json!(schemars::schema_for!(Project));
    let budget = &schema["$defs"]["AgentBudgetSettings"];
    assert_eq!(budget["additionalProperties"], false);
    assert_eq!(budget["required"], json!(["max_tokens"]));
    assert_eq!(budget["properties"]["max_tokens"]["type"], "integer");
    assert_eq!(budget["properties"]["max_tokens"]["minimum"], 0);
    assert_eq!(
        budget["properties"]["max_tokens"]["maximum"],
        MAX_AGENT_TOKEN_CAP
    );
    let settings = &schema["$defs"]["ProjectSettings"];
    assert_eq!(
        settings["properties"]["agent_budget"]["default"],
        json!({"max_tokens": 1_000_000})
    );
    assert!(
        !settings["required"]
            .as_array()
            .unwrap()
            .contains(&json!("agent_budget"))
    );
}
