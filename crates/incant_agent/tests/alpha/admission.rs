use super::support::{Fixture, TOOL, actor};
use incant_agent::{NoViewport, dispatch, tools};
use incant_cmd::{Command, CommandBus};
use incant_doc::Project;
use serde_json::{Value, json};
use std::fs;

#[test]
fn strict_args_reject_extra_authority_missing_revision_and_invalid_selection() {
    let mut f = Fixture::new();
    let mut host = f.host();
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let valid = f.args();
    let mut candidates = vec![Value::Null, json!({}), valid.clone()];
    candidates[2]
        .as_object_mut()
        .unwrap()
        .remove("expected_revision");
    for key in ["root", "cache", "source", "actor", "description", "execute"] {
        let mut bad = valid.clone();
        bad[key] = json!("untrusted");
        candidates.push(bad);
        let mut nested = valid.clone();
        nested["selection"][key] = json!("untrusted");
        candidates.push(nested);
    }
    for threshold in [json!(0), json!(256), json!(-1), json!(1.5), json!("128")] {
        let mut bad = valid.clone();
        bad["selection"]["threshold"] = threshold;
        candidates.push(bad);
    }
    for key in ["scene_id", "entity_id", "threshold"] {
        let mut bad = valid.clone();
        bad["selection"].as_object_mut().unwrap().remove(key);
        candidates.push(bad);
    }
    for key in ["scene_id", "entity_id"] {
        let mut bad = valid.clone();
        bad["selection"][key] = json!("../outside");
        candidates.push(bad);
    }
    let mut stale = valid.clone();
    stale["expected_revision"] = json!(f.bus.revision() + 1);
    candidates.push(stale);
    for args in candidates {
        assert!(
            dispatch(&mut f.bus, &mut host, TOOL, args.clone(), actor()).is_err(),
            "{args}"
        );
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.history(), history);
    }
    let definition = tools().into_iter().find(|v| v["name"] == TOOL).unwrap();
    let schema = &definition["parameters"];
    assert_eq!(schema["additionalProperties"], false);
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("expected_revision"))
    );
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("selection"))
    );
}

#[test]
fn host_owner_unsupported_sprite_and_missing_cooked_cache_never_publish() {
    let mut f = Fixture::new();
    let args = f.args();
    assert!(dispatch(&mut f.bus, &mut NoViewport, TOOL, args, actor()).is_err());
    let mut host = f.host();
    let mut other = CommandBus::new(Project::empty("Other")).unwrap();
    let mut other_args = f.args();
    other_args["expected_revision"] = json!(0);
    assert!(dispatch(&mut other, &mut host, TOOL, other_args, actor()).is_err());
    assert!(other.history().is_empty());
    let mut sprite =
        f.bus.project().scenes[&f.scene].entities[&f.entity].components["SpriteRenderer"].clone();
    sprite["tint"] = json!([1, 1, 1, 0.5]);
    f.bus
        .execute(
            vec![Command::SetComponent {
                scene_id: f.scene.clone(),
                entity_id: f.entity.clone(),
                component: "SpriteRenderer".into(),
                value: sprite,
            }],
            actor(),
            "Unsupported tint",
            None,
        )
        .unwrap();
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let args = f.args();
    assert!(dispatch(&mut f.bus, &mut host, TOOL, args, actor()).is_err());
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);

    let mut f = Fixture::new();
    let mut host = f.host();
    fs::remove_dir_all(f.root.path().join(".incant/cache/textures")).unwrap();
    let before = f.bus.project().clone();
    let args = f.args();
    assert!(dispatch(&mut f.bus, &mut host, TOOL, args, actor()).is_err());
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), 2);
}

#[cfg(unix)]
#[test]
fn default_cache_redirect_is_refused_without_reading_an_arbitrary_root() {
    let mut f = Fixture::new();
    let mut host = f.host();
    let outside = tempfile::tempdir().unwrap();
    let cache = f.root.path().join(".incant/cache/textures");
    fs::rename(&cache, outside.path().join("textures")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("textures"), &cache).unwrap();
    let before = f.bus.project().clone();
    let args = f.args();
    assert!(dispatch(&mut f.bus, &mut host, TOOL, args, actor()).is_err());
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), 2);
}
