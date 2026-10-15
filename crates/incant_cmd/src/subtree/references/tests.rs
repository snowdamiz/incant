use super::*;
use serde_json::json;
use std::collections::BTreeSet;

fn string_paths(value: &Value, path: &str, output: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            let string = value["type"] == "string"
                || value["type"]
                    .as_array()
                    .is_some_and(|a| a.contains(&json!("string")));
            if string && !object.contains_key("enum") && !object.contains_key("const") {
                output.insert(path.into());
            }
            match object.get("additionalProperties") {
                Some(Value::Bool(true)) => {
                    output.insert(format!("{path}/<literal-json>"));
                }
                Some(Value::Object(_)) => {
                    output.insert(format!("{path}/<map-key>"));
                }
                _ => {}
            }
            for (key, child) in object {
                let key = key.replace('~', "~0").replace('/', "~1");
                string_paths(child, &format!("{path}/{key}"), output);
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                string_paths(child, &format!("{path}/{index}"), output);
            }
        }
        _ => {}
    }
}

#[test]
fn every_component_and_non_enum_string_slot_has_an_audited_reference_class() {
    let mut schemas = incant_doc::schema_registry();
    for metadata in [
        "Project",
        "Prefab",
        "PrefabLink",
        "EngineCompatibility",
        "SpriteAtlas",
    ] {
        assert!(schemas.remove(metadata).is_some());
    }
    let names: BTreeSet<_> = LITERAL_COMPONENTS
        .iter()
        .copied()
        .chain(["AudioBus", "AudioSource", "NavigationMesh"])
        .collect();
    assert_eq!(
        schemas.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        names
    );
    let actual: BTreeMap<_, _> = schemas
        .iter()
        .map(|(name, schema)| {
            let mut paths = BTreeSet::new();
            string_paths(schema, "", &mut paths);
            (name.clone(), paths)
        })
        .collect();
    let expected: BTreeMap<String, BTreeSet<String>> =
        serde_json::from_str(include_str!("inventory.json")).unwrap();
    assert_eq!(
        actual, expected,
        "review new reference slots before updating the inventory"
    );
}

#[test]
fn typed_internal_edges_change_without_reserializing_other_values() {
    let map = BTreeMap::from([("internal".into(), "new".into())]);
    let components = BTreeMap::from([
        ("AudioBus".into(), json!({"parent":"internal","gain_db":0})),
        (
            "AudioSource".into(),
            json!({"clip":"internal","bus":"external","gain_db":0,
            "pan":0,"rate":1,"start_seconds":0,"playing":false,"looping":false,"streaming":false}),
        ),
        (
            "NavigationMesh".into(),
            json!({"settings":{"min":[0,0,0],"max":[10,10,10],
            "cell_size":0.1,"cell_height":0.1,"tile_cells":16,"agent_radius":0.2,
            "agent_height":1,"max_climb":0.3,"max_slope_degrees":45},
            "sources":[{"entity":"internal","geometry":"collider"},
                       {"entity":"external","geometry":"collider"}]}),
        ),
        (
            "Script".into(),
            json!({"source":"internal","props":{"entity":"internal","array":["internal"]}}),
        ),
        (
            "AnimationGraph".into(),
            json!({"initial":"internal","parameters":{},
            "states":{"internal":{"motion":{"kind":"clip","clip":0},"looping":true}},"transitions":{}}),
        ),
    ]);
    let mut expected = components.clone();
    expected.get_mut("AudioBus").unwrap()["parent"] = json!("new");
    expected.get_mut("NavigationMesh").unwrap()["sources"][0]["entity"] = json!("new");
    assert_eq!(remap(&components, &map).unwrap(), expected);
    let omitted = BTreeMap::from([("AudioBus".into(), json!({"gain_db":0}))]);
    assert_eq!(remap(&omitted, &map).unwrap(), omitted);
    for bad in [
        json!({"parent":7,"gain_db":0}),
        json!({"parent":"internal","gain_db":0,"extra":true}),
    ] {
        assert!(remap(&BTreeMap::from([("AudioBus".into(), bad)]), &map).is_err());
    }
    assert!(
        remap(
            &BTreeMap::from([("FutureComponent".into(), json!({}))]),
            &map
        )
        .is_err()
    );
}
