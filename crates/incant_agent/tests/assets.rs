use incant_agent::{
    Agent, AgentError, ApprovalMode, Budget, NoViewport, ProjectHost, Usage, dispatch,
    provider::{Completion, Provider},
};
use incant_cmd::{Actor, CommandBus};
use incant_doc::{Origin, Project};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    fs,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

fn fixture(root: &Path, name: &str) {
    fs::write(
        root.join("mesh.bin"),
        [0_f32, 0., 0., 1., 0., 0., 0., 1., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    fs::write(root.join(name), json!({"asset":{"version":"2.0"},"buffers":[{"uri":"mesh.bin","byteLength":36}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}).to_string()).unwrap();
}
fn args(revision: u64, sources: &[&str]) -> Value {
    json!({"sources":sources.iter().map(|source| json!({"source":source})).collect::<Vec<_>>(),"expected_revision":revision,"description":"Import local models"})
}
fn actor() -> Actor {
    Actor::agent("fixture-model", "asset-session")
}

#[test]
fn imported_assets_have_real_cooked_content_shared_history_and_paged_inspection() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root, "a.gltf");
    fixture(root, "b.gltf");
    let initial = Project::empty("Agent assets");
    let journal = root.join("journal");
    let mut bus = CommandBus::persistent(&journal, initial.clone()).unwrap();
    let mut host = ProjectHost::new(&initial, root, NoViewport).unwrap();
    let imported = dispatch(
        &mut bus,
        &mut host,
        "asset_import",
        args(0, &["a.gltf", "b.gltf"]),
        actor(),
    )
    .unwrap();
    assert_eq!(imported["revision"], 1);
    assert_eq!(imported["transaction_id"], bus.history()[0].id);
    assert_eq!(bus.history()[0].commands.len(), 2);
    assert_eq!(bus.history()[0].actor.origin, Origin::Agent);
    assert_eq!(
        bus.history()[0].actor.model.as_deref(),
        Some("fixture-model")
    );
    assert_eq!(
        bus.history()[0].actor.conversation_id.as_deref(),
        Some("asset-session")
    );
    for asset in bus.project().assets.values() {
        assert!(
            root.join(format!(".incant/cache/models/{}.incmodel", asset.sha256))
                .is_file()
        );
    }
    let repeated = dispatch(
        &mut bus,
        &mut host,
        "asset_import",
        args(1, &["a.gltf"]),
        actor(),
    )
    .unwrap();
    assert!(repeated["transaction_id"].is_null());
    assert_eq!(repeated["untrusted_project_data"][0]["changed"], false);
    let first = dispatch(
        &mut bus,
        &mut NoViewport,
        "asset_list",
        json!({"limit":1,"filter":"MODEL"}),
        actor(),
    )
    .unwrap();
    let cursor = first["next_after"].as_str().unwrap();
    let second = dispatch(
        &mut bus,
        &mut NoViewport,
        "asset_list",
        json!({"limit":1,"after":cursor}),
        actor(),
    )
    .unwrap();
    assert!(second["next_after"].is_null());
    assert_ne!(
        first["untrusted_project_data"][0]["id"],
        second["untrusted_project_data"][0]["id"]
    );
    let inspected = dispatch(
        &mut bus,
        &mut host,
        "asset_inspect",
        json!({"id":cursor}),
        actor(),
    )
    .unwrap();
    assert_eq!(
        inspected["untrusted_project_data"],
        first["untrusted_project_data"][0]
    );
    assert!(
        dispatch(
            &mut bus,
            &mut host,
            "asset_list",
            json!({"limit":0}),
            actor()
        )
        .is_err()
    );
    assert!(
        dispatch(
            &mut bus,
            &mut host,
            "asset_inspect",
            json!({"id":"missing"}),
            actor()
        )
        .is_err()
    );
    let saved = bus.project().clone();
    drop(bus);
    let mut recovered = CommandBus::persistent(&journal, saved).unwrap();
    recovered.undo().unwrap();
    assert_eq!(recovered.project(), &initial);
    recovered.redo().unwrap();
    assert_eq!(recovered.project().assets.len(), 2);
}

#[test]
fn imports_require_host_access_current_revision_and_confined_valid_batches() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root, "a.gltf");
    let mut bus = CommandBus::new(Project::empty("Limits")).unwrap();
    assert!(
        dispatch(
            &mut bus,
            &mut NoViewport,
            "asset_import",
            args(0, &["a.gltf"]),
            actor()
        )
        .is_err()
    );
    let mut host = ProjectHost::new(bus.project(), root, NoViewport).unwrap();
    assert!(
        dispatch(
            &mut bus,
            &mut host,
            "asset_import",
            args(1, &["a.gltf"]),
            actor()
        )
        .is_err()
    );
    assert!(!root.join(".incant/cache").exists());
    for source in [
        "../a.gltf",
        "/a.gltf",
        "https://example.com/a.gltf",
        "./a.gltf",
    ] {
        assert!(
            dispatch(
                &mut bus,
                &mut host,
                "asset_import",
                args(0, &[source]),
                actor()
            )
            .is_err()
        );
    }
    fs::write(root.join("broken.gltf"), b"broken").unwrap();
    assert!(
        dispatch(
            &mut bus,
            &mut host,
            "asset_import",
            args(0, &["a.gltf", "broken.gltf"]),
            actor()
        )
        .is_err()
    );
    assert!(bus.project().assets.is_empty());
    assert!(bus.history().is_empty());
    let mut other = CommandBus::new(Project::empty("Other project")).unwrap();
    assert!(
        dispatch(
            &mut other,
            &mut host,
            "asset_import",
            args(0, &["a.gltf"]),
            actor()
        )
        .is_err()
    );
    let mut invalid = args(0, &["a.gltf"]);
    invalid["root"] = json!("/private");
    assert!(dispatch(&mut bus, &mut host, "asset_import", invalid, actor()).is_err());
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        fixture(outside.path(), "outside.gltf");
        std::os::unix::fs::symlink(
            outside.path().join("outside.gltf"),
            root.join("outside.gltf"),
        )
        .unwrap();
        assert!(
            dispatch(
                &mut bus,
                &mut host,
                "asset_import",
                args(0, &["outside.gltf"]),
                actor()
            )
            .is_err()
        );
    }
}

struct Scripted {
    outputs: VecDeque<Vec<Value>>,
    advertised_import: Vec<bool>,
}
impl Provider for Scripted {
    fn model(&self) -> &str {
        "fixture-model"
    }
    fn complete(
        &mut self,
        _: &[Value],
        tools: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        self.advertised_import
            .push(tools.iter().any(|tool| tool["name"] == "asset_import"));
        Ok(Completion {
            output: self.outputs.pop_front().unwrap(),
            usage: Usage {
                input_tokens: 20,
                output_tokens: 10,
            },
        })
    }
}
fn provider() -> Scripted {
    Scripted {
        outputs: VecDeque::from([
            vec![
                json!({"type":"function_call","name":"asset_import","call_id":"import1","arguments":args(0, &["a.gltf"]).to_string()}),
            ],
            vec![],
        ]),
        advertised_import: vec![],
    }
}
fn agent() -> Agent {
    Agent {
        budget: Budget {
            max_tokens: 100_000,
            used_tokens: 0,
        },
        approval: ApprovalMode::Destructive,
        max_steps: 3,
        max_output_tokens: 25_000,
    }
}

#[test]
fn agent_approval_and_cancellation_precede_import_and_unscoped_hosts_do_not_advertise_it() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root, "a.gltf");
    let mut bus = CommandBus::new(Project::empty("Approvals")).unwrap();
    let mut host = ProjectHost::new(bus.project(), root, NoViewport).unwrap();
    let cancelled = AtomicBool::new(false);
    let mut asked = 0;
    let denied = agent()
        .run(
            &mut provider(),
            &mut bus,
            &mut host,
            "Import a.gltf",
            "session",
            &cancelled,
            &mut |name, _| {
                assert_eq!(name, "asset_import");
                asked += 1;
                false
            },
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(asked, 1);
    assert!(!denied.tool_calls[0].succeeded);
    assert!(!root.join(".incant/cache").exists());
    let result = agent().run(
        &mut provider(),
        &mut bus,
        &mut host,
        "Import a.gltf",
        "session",
        &cancelled,
        &mut |_, _| {
            cancelled.store(true, Ordering::Relaxed);
            true
        },
        &mut |_| {},
    );
    assert!(matches!(result, Err(AgentError::Interrupted)));
    assert!(!root.join(".incant/cache").exists());
    assert!(bus.history().is_empty());
    cancelled.store(false, Ordering::Relaxed);
    let report = agent()
        .run(
            &mut provider(),
            &mut bus,
            &mut host,
            "Import a.gltf",
            "session",
            &cancelled,
            &mut |_, _| true,
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(report.transaction_ids.len(), 1);
    assert!(report.tool_calls[0].succeeded);
    let mut unscoped = provider();
    agent()
        .run(
            &mut unscoped,
            &mut bus,
            &mut NoViewport,
            "Try import",
            "session",
            &cancelled,
            &mut |_, _| true,
            &mut |_| {},
        )
        .unwrap();
    assert!(
        unscoped
            .advertised_import
            .iter()
            .all(|available| !available)
    );
}
