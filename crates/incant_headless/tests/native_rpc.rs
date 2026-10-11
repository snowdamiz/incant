use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, Transform};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command as Process, Stdio},
};

fn fixture(root: &Path) -> (String, String) {
    let mut project = Project::empty("Native RPC");
    project.settings.tick_rate = 10;
    let scene = Scene::new("Main");
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    let mut bus = CommandBus::new(project).unwrap();
    let mut entity = Entity::new("Mover");
    let eid = entity.id.clone();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Velocity".into(), json!({"linear":[2.,0.,0.]}));
    bus.execute(
        vec![
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity,
            },
            Command::SetInputActions {
                actions: serde_json::from_value(json!({
                    "right":{"kind":"button","bindings":[{"type":"key","code":"KeyD"}]}
                }))
                .unwrap(),
            },
        ],
        Actor::user("test"),
        "Fixture",
        None,
    )
    .unwrap();
    fs::write(
        root.join("project.json"),
        bus.project().canonical_text().unwrap(),
    )
    .unwrap();
    (sid, eid)
}
fn call(method: &str, params: Value) -> Value {
    json!({"method":method,"params":params})
}
fn rpc(root: &Path, calls: Vec<Value>) -> Vec<Value> {
    let before = fs::read(root.join("project.json")).unwrap();
    let mut process = Process::new(env!("CARGO_BIN_EXE_incant_headless"))
        .args(["rpc", "project.json"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = process.stdin.take().unwrap();
    // Small protocol cases stay below pipe capacity. Assign IDs to verify that a
    // rejected request does not desynchronize subsequent commands in the process.
    let count = calls.len();
    for (id, mut request) in calls.into_iter().enumerate() {
        request["id"] = json!(id);
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);
    let output = process.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(before, fs::read(root.join("project.json")).unwrap());
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(responses.len(), count);
    for (id, response) in responses.iter().enumerate() {
        assert_eq!(response["id"], id);
    }
    responses
}
fn ok(value: &Value) -> &Value {
    assert!(value.get("error").is_none(), "{value}");
    value.get("result").unwrap()
}
fn error(value: &Value, text: &str) {
    assert!(
        value["error"]["message"]
            .as_str()
            .is_some_and(|s| s.contains(text)),
        "{value}"
    );
    assert!(value.get("result").is_none());
}

#[test]
fn retained_native_motion_input_and_authoring_are_independent() {
    let temp = tempfile::tempdir().unwrap();
    let (sid, eid) = fixture(temp.path());
    let missing = incant_doc::new_id();
    let response = rpc(
        temp.path(),
        vec![
            json!({"method":"play.start"}),
            call(
                "play.step",
                json!({"events":[{"type":"key","code":"KeyD","down":true}],
            "observe":{"entities":[eid,missing],"include_input":true,"include_logs":true}}),
            ),
            call(
                "command.execute",
                json!({"expected_revision":0,"description":"Edit authoring during play",
            "commands":[{"op":"set_component","scene_id":sid,"entity_id":eid,
                "component":"Velocity","value":{"linear":[9,0,0]}}]}),
            ),
            call(
                "play.step",
                json!({"observe":{"entities":[eid],"include_input":true}}),
            ),
            call("play.state", json!({})),
            call("play.stop", json!({})),
            call("play.start", json!({})),
            call("play.step", json!({"observe":{"entities":[eid]}})),
            call("play.stop", json!({})),
            call("history.read", json!({})),
        ],
    );
    assert_eq!(ok(&response[0])["runtime"], "native");
    assert_eq!(ok(&response[0])["script_source_parsing"], false);
    assert_eq!(ok(&response[0])["tick"], 0);
    assert_eq!(ok(&response[0])["entities"], json!({}));
    let first = ok(&response[1]);
    assert_eq!(
        first["entities"][&eid]["transform"]["translation"],
        json!([0.2, 0., 0.])
    );
    assert_eq!(first["entities"][&missing], Value::Null);
    assert_eq!(first["input"]["actions"]["right"]["pressed"], true);
    assert_eq!(first["logs"]["entries"], json!([]));
    assert_eq!(first["frames"], 4800);
    assert_eq!(ok(&response[2])["revision"], 1);
    let second = ok(&response[3]);
    assert_eq!(second["started_revision"], 0);
    assert_eq!(
        second["entities"][&eid]["transform"]["translation"],
        json!([0.4, 0., 0.])
    );
    assert_eq!(second["input"]["actions"]["right"]["pressed"], false);
    assert_eq!(second["input"]["actions"]["right"]["active"], true);
    let state = ok(&response[4]);
    assert_eq!(state["tick"], 2);
    assert_eq!(state["entities"], json!({}));
    assert!(state.get("input").is_none() && state.get("logs").is_none());
    assert_eq!(ok(&response[5])["revision"], 1);
    assert_eq!(ok(&response[6])["started_revision"], 1);
    assert_eq!(
        ok(&response[7])["entities"][&eid]["transform"]["translation"],
        json!([0.9, 0., 0.])
    );
    assert_eq!(ok(&response[9]).as_array().unwrap().len(), 1);
}

#[test]
fn invalid_requests_and_input_preserve_tick_and_live_session() {
    let temp = tempfile::tempdir().unwrap();
    let (_, eid) = fixture(temp.path());
    let response = rpc(
        temp.path(),
        vec![
            call("play.step", json!({})),
            call("play.start", json!({})),
            call("play.start", json!({})),
            call("play.step", json!({"unknown":true})),
            call("play.step", json!({"observe":{"entities":[eid,eid]}})),
            call("play.step", json!({"observe":{"entities":["bad"]}})),
            call(
                "play.step",
                json!({"observe":{"entities":vec![eid.clone();257]}}),
            ),
            call(
                "play.step",
                json!({"events":[{"type":"gamepad_axis","id":0,"axis":"left_x","value":0.5}]}),
            ),
            call("play.state", json!({})),
            call("play.stop", json!({"keep":true})),
            call("play.step", json!({"observe":{"entities":[eid]}})),
            call("play.stop", json!({})),
            call("play.state", json!({})),
            call("play.stop", json!({})),
        ],
    );
    error(&response[0], "no play session");
    ok(&response[1]);
    error(&response[2], "stop the existing");
    error(&response[3], "unknown field");
    error(&response[4], "duplicate");
    error(&response[5], "canonical");
    error(&response[6], "at most 256");
    error(&response[7], "not connected");
    assert_eq!(ok(&response[8])["tick"], 0);
    error(&response[9], "unknown field");
    assert_eq!(ok(&response[10])["tick"], 1);
    ok(&response[11]);
    error(&response[12], "no play session");
    ok(&response[13]);
}

#[test]
fn failed_runtime_hides_old_state_and_stop_permits_restart() {
    let temp = tempfile::tempdir().unwrap();
    let (sid, eid) = fixture(temp.path());
    let response = rpc(
        temp.path(),
        vec![
            call(
                "command.execute",
                json!({"expected_revision":0,"description":"Overflow fixture","commands":[
                    {"op":"set_component","scene_id":sid,"entity_id":eid,"component":"Transform","value":{
                        "translation":[1.6e308,0.,0.],"rotation":[0.,0.,0.,1.],"scale":[1.,1.,1.]}},
                    {"op":"set_component","scene_id":sid,"entity_id":eid,"component":"Velocity","value":{"linear":[1e308,0.,0.]}}
                ]}),
            ),
            call("play.start", json!({})),
            call("play.step", json!({})),
            call("play.step", json!({})),
            call(
                "play.state",
                json!({"entities":[eid],"include_input":true,"include_logs":true}),
            ),
            call("play.step", json!({})),
            call("play.stop", json!({})),
            call("play.start", json!({})),
        ],
    );
    ok(&response[0]);
    ok(&response[1]);
    assert_eq!(ok(&response[2])["tick"], 1);
    assert!(response[3].get("error").is_some());
    error(&response[4], "failed");
    error(&response[5], "failed");
    ok(&response[6]);
    assert_eq!(ok(&response[7])["tick"], 0);
}

#[test]
fn invalid_startup_never_installs_a_session() {
    let temp = tempfile::tempdir().unwrap();
    fixture(temp.path());
    let response = rpc(
        temp.path(),
        vec![
            call("play.start", json!({"sample_rate":0})),
            call("play.start", json!({"scene":"bad"})),
            call("play.start", json!({"script_sha256":"0".repeat(64)})),
            call("play.start", json!({"script":"untrusted.js"})),
            call("play.state", json!({})),
            call("play.start", json!({"sample_rate":44117})),
            call("play.step", json!({})),
        ],
    );
    error(&response[0], "sample rate");
    error(&response[1], "canonical");
    error(&response[2], "source hash");
    error(&response[3], "unknown field");
    error(&response[4], "no play session");
    assert_eq!(ok(&response[6])["frames"], 4411);
}

#[test]
fn project_script_does_not_fall_back_to_legacy_play() {
    let temp = tempfile::tempdir().unwrap();
    let (sid, eid) = fixture(temp.path());
    let path = temp.path().join("project.json");
    let mut project = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let source_id = incant_doc::new_id();
    project.scripts.insert(
        source_id.clone(),
        incant_doc::ScriptSource {
            id: source_id.clone(),
            name: "Unmigrated".into(),
            path: "scripts/legacy.ts".into(),
        },
    );
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert("Script".into(), json!({"source":source_id,"props":{}}));
    fs::write(path, project.canonical_text().unwrap()).unwrap();
    let response = rpc(
        temp.path(),
        vec![call("play.start", json!({})), call("play.state", json!({}))],
    );
    error(&response[0], "Script");
    error(&response[1], "no play session");
}

#[test]
fn bounded_run_and_state_query_keep_completed_world_across_authoring_and_failed_candidates() {
    let temp = tempfile::tempdir().unwrap();
    let (sid, eid) = fixture(temp.path());
    let selected = format!("/entities/{eid}/transform/translation");
    let response = rpc(
        temp.path(),
        vec![
            call("play.run", json!({"seconds":0.5})),
            call("play.state_query", json!({"path":selected})),
            call(
                "command.execute",
                json!({"expected_revision":0,"description":"Overflow next run","commands":[
                    {"op":"set_component","scene_id":sid,"entity_id":eid,"component":"Transform","value":{
                        "translation":[1.6e308,0.,0.],"rotation":[0.,0.,0.,1.],"scale":[1.,1.,1.]}},
                    {"op":"set_component","scene_id":sid,"entity_id":eid,"component":"Velocity","value":{"linear":[1e308,0.,0.]}}
                ]}),
            ),
            call("play.state_query", json!({"path":selected})),
            call("play.run", json!({"seconds":0.5})),
            call("play.state_query", json!({"path":selected})),
            call("play.state_query", json!({"path":"/entities"})),
            call("play.state_query", json!({"path":"/input"})),
            call("play.stop", json!({})),
            call("play.state_query", json!({"path":"/tick"})),
        ],
    );
    assert_eq!(ok(&response[0])["state"]["tick"], 5);
    assert_eq!(ok(&response[0])["passed"], true);
    assert_eq!(
        ok(&response[1])["untrusted_runtime_data"],
        json!([1., 0., 0.])
    );
    ok(&response[2]);
    assert_eq!(ok(&response[1]), ok(&response[3]));
    assert!(response[4].get("error").is_some());
    assert_eq!(ok(&response[1]), ok(&response[5]));
    error(&response[6], "select one entity");
    assert_eq!(ok(&response[7])["tick"], 5);
    ok(&response[8]);
    error(&response[9], "no play session");
}
