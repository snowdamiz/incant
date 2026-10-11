use super::*;
use incant_agent::{
    Agent, ApprovalMode, Budget, NoViewport, ProjectHost, Usage, dispatch,
    provider::{Completion, Provider},
};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Scene, Transform};
use serde_json::json;
use std::{collections::VecDeque, sync::atomic::Ordering};

fn fixture() -> (CommandBus, String, String) {
    let mut project = Project::empty("Native agent play");
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
        vec![Command::CreateEntity {
            scene_id: sid.clone(),
            entity,
        }],
        Actor::user("test"),
        "Fixture",
        None,
    )
    .unwrap();
    (bus, sid, eid)
}
fn run_request(eid: &str, expected: f64) -> Value {
    json!({"seconds":0.5,"inputs_script":{"format":"incant-input","version":1,"tick_rate":10,"start_tick":0,"ticks":5,"frames":[{"tick":1,"events":[{"type":"key","code":"KeyD","down":true}]}]},
    "observe":{"format":"incant-native-observation","version":1,"entities":[eid],"checks":[{"name":"motion","tick":5,"expect":{"type":"vector","entity":eid,"property":"translation","value":[expected,0.,0.],"tolerance":1e-10}}]}})
}
fn call(
    bus: &mut CommandBus,
    host: &mut dyn EngineHost,
    name: &str,
    args: Value,
) -> Result<Value, AgentError> {
    dispatch(bus, host, name, args, Actor::agent("fixture", "session"))
}
fn agent() -> Agent {
    Agent {
        budget: Budget {
            max_tokens: 250_000,
            used_tokens: 0,
        },
        approval: ApprovalMode::Destructive,
        max_steps: 4,
        max_output_tokens: 1000,
    }
}
struct Scripted {
    outputs: VecDeque<Vec<Value>>,
    inputs: Vec<Vec<Value>>,
    advertised: Vec<bool>,
}
impl Provider for Scripted {
    fn model(&self) -> &str {
        "fixture"
    }
    fn complete(
        &mut self,
        input: &[Value],
        tools: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        self.inputs.push(input.to_vec());
        self.advertised
            .push(tools.iter().any(|tool| tool["name"] == "play_run"));
        Ok(Completion {
            output: self.outputs.pop_front().unwrap(),
            usage: Usage {
                input_tokens: 20,
                output_tokens: 10,
            },
        })
    }
}

#[test]
fn provider_loop_runs_native_motion_and_queries_actual_state_without_authoring_edits() {
    let temp = tempfile::tempdir().unwrap();
    let (mut bus, _, eid) = fixture();
    let before = bus.project().canonical_text().unwrap();
    let revision = bus.revision();
    let mut host = ProjectHost::new(
        bus.project(),
        temp.path(),
        Host::new(temp.path().to_path_buf(), NoViewport),
    )
    .unwrap();
    let path = format!("/entities/{eid}/transform/translation");
    let mut provider = Scripted {
        outputs: VecDeque::from([
            vec![
                json!({"type":"function_call","name":"play_run","call_id":"run","arguments":run_request(&eid,1.).to_string()}),
            ],
            vec![
                json!({"type":"function_call","name":"play_state_query","call_id":"state","arguments":json!({"path":path}).to_string()}),
            ],
            vec![],
        ]),
        inputs: vec![],
        advertised: vec![],
    };
    let report = agent()
        .run(
            &mut provider,
            &mut bus,
            &mut host,
            "Run and inspect motion",
            "session",
            &AtomicBool::new(false),
            &mut |_, _| panic!("read-only play needs no destructive approval"),
            &mut |_| {},
        )
        .unwrap();
    assert!(report.tool_calls.iter().all(|call| call.succeeded));
    assert!(provider.advertised.iter().all(|available| *available));
    let results: Vec<Value> = provider
        .inputs
        .last()
        .unwrap()
        .iter()
        .filter(|v| v["type"] == "function_call_output")
        .map(|v| serde_json::from_str(v["output"].as_str().unwrap()).unwrap())
        .collect();
    assert_eq!(results[0]["passed"], true);
    assert_eq!(results[0]["state"]["tick"], 5);
    assert_eq!(results[0]["state"]["frames"], 24000);
    assert_eq!(results[0]["state"]["script_source_parsing"], false);
    assert_eq!(results[0]["state"]["gpu_submitted"], false);
    assert_eq!(results[1]["untrusted_runtime_data"], json!([1., 0., 0.]));
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    assert_eq!(bus.revision(), revision);
    assert!(report.transaction_ids.is_empty());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn completed_false_assertion_is_inspectable_and_bad_runs_preserve_previous_session() {
    let temp = tempfile::tempdir().unwrap();
    let (mut bus, _, eid) = fixture();
    let mut host = Host::new(temp.path().to_path_buf(), NoViewport);
    let result = call(&mut bus, &mut host, "play_run", run_request(&eid, 99.)).unwrap();
    assert_eq!(result["passed"], false);
    let path = json!({"path":format!("/entities/{eid}/transform/translation")});
    let before = call(&mut bus, &mut host, "play_state_query", path.clone()).unwrap();
    let mut invalid = run_request(&eid, 1.);
    invalid["inputs_script"]["frames"][0]["events"][0] =
        json!({"type":"gamepad_axis","id":0,"axis":"left_x","value":0.5});
    assert!(call(&mut bus, &mut host, "play_run", invalid).is_err());
    let mut invalid = run_request(&eid, 1.);
    invalid["script_sha256"] = json!("00".repeat(32));
    assert!(call(&mut bus, &mut host, "play_run", invalid).is_err());
    assert!(call(&mut bus, &mut host, "play_run", json!({"seconds":0})).is_err());
    assert!(call(&mut bus, &mut host, "play_run", json!({"seconds":601})).is_err());
    assert!(
        call(
            &mut bus,
            &mut host,
            "play_run",
            json!({"seconds":1,"command":"whoami"})
        )
        .is_err()
    );
    assert!(matches!(
        host.play_run(
            bus.project(),
            bus.revision(),
            serde_json::from_value(run_request(&eid, 1.)).unwrap(),
            &AtomicBool::new(true)
        ),
        Err(AgentError::Interrupted)
    ));
    assert_eq!(
        call(&mut bus, &mut host, "play_state_query", path).unwrap(),
        before
    );
    for path in [
        "",
        "/",
        "/entities",
        "/entities/not-an-id",
        "/project",
        "/tick/missing",
    ] {
        assert!(
            call(
                &mut bus,
                &mut host,
                "play_state_query",
                json!({"path":path})
            )
            .is_err(),
            "{path}"
        );
    }
}

#[test]
fn unsupported_hosts_hide_play_and_always_approval_can_cancel_before_execution() {
    let temp = tempfile::tempdir().unwrap();
    let (mut bus, _, eid) = fixture();
    let mut no_play = Scripted {
        outputs: VecDeque::from([vec![]]),
        inputs: vec![],
        advertised: vec![],
    };
    agent()
        .run(
            &mut no_play,
            &mut bus,
            &mut NoViewport,
            "Hello",
            "session",
            &AtomicBool::new(false),
            &mut |_, _| true,
            &mut |_| {},
        )
        .unwrap();
    assert_eq!(no_play.advertised, vec![false]);
    let mut host = Host::new(temp.path().to_path_buf(), NoViewport);
    let cancel = AtomicBool::new(false);
    let mut provider = Scripted {
        outputs: VecDeque::from([vec![
            json!({"type":"function_call","name":"play_run","call_id":"run","arguments":run_request(&eid,1.).to_string()}),
        ]]),
        inputs: vec![],
        advertised: vec![],
    };
    let mut agent = agent();
    agent.approval = ApprovalMode::Always;
    let result = agent.run(
        &mut provider,
        &mut bus,
        &mut host,
        "Run",
        "session",
        &cancel,
        &mut |_, _| {
            cancel.store(true, Ordering::Relaxed);
            true
        },
        &mut |_| {},
    );
    assert!(matches!(result, Err(AgentError::Interrupted)));
    assert!(host.play_state_query("/tick").is_err());
}
