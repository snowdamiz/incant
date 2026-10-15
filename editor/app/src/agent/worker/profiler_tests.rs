//! Real editor-owned CPU host/lifecycle; provider transport is a local fixture.
//! No account, network, viewport request, device/window or capture is invoked.
use super::*;
use crate::agent::Service;
use incant_agent::{Usage, provider::Completion, session::*};
use incant_cmd::CommandBus;
use incant_doc::{Project, Scene};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

type Action = (&'static str, Value, bool);
struct Scripted {
    actions: Vec<Action>,
    next: usize,
    replies: Arc<Mutex<Vec<Value>>>,
    profiler: bool,
}
impl Provider for Scripted {
    fn model(&self) -> &str {
        "local-profiler-transport-fixture"
    }
    fn complete(
        &mut self,
        input: &[Value],
        definitions: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        assert_eq!(
            definitions.iter().any(|v| v["name"] == "profiler_read"),
            self.profiler
        );
        if self.next > 0 {
            let reply: Value =
                serde_json::from_str(input.last().unwrap()["output"].as_str().unwrap()).unwrap();
            assert_eq!(reply.get("error").is_none(), self.actions[self.next - 1].2);
            self.replies.lock().unwrap().push(reply);
        }
        let output = if let Some((name, args, _)) = self.actions.get(self.next) {
            vec![
                json!({"type":"function_call", "call_id":format!("action{}",self.next),
                "name":name,"arguments":args.to_string()}),
            ]
        } else {
            Vec::new()
        };
        self.next += 1;
        Ok(Completion {
            output,
            usage: Usage::default(),
        })
    }
}
fn request() -> TurnRequest {
    TurnRequest {
        prompt: "Measure the accepted CPU play session".into(),
        model: "local-profiler-transport-fixture".into(),
        approval: ApprovalChoice::Auto,
        view_context: None,
    }
}
fn project() -> Project {
    let mut project = Project::empty("Owned profile");
    project.settings.tick_rate = 10;
    let scene = Scene::new("Main");
    project.scenes.insert(scene.id.clone(), scene);
    project
}
fn load(project: Project) -> Arc<Mutex<crate::project::LoadState>> {
    Arc::new(Mutex::new(crate::project::LoadState::Ready(Box::new(
        CommandBus::new(project).unwrap(),
    ))))
}
fn turn(
    service: &Service,
    bus: &Arc<Mutex<crate::project::LoadState>>,
    root: Option<PathBuf>,
    account: &str,
    actions: Vec<Action>,
) -> Vec<Value> {
    let replies = Arc::new(Mutex::new(Vec::new()));
    let provider = Scripted {
        actions,
        next: 0,
        replies: replies.clone(),
        profiler: root.is_some(),
    };
    let owned = bus.clone();
    let account = Connection::OAuth(account.into());
    service
        .start(
            request(),
            || Ok(account),
            move |turn, connection, owner| {
                owner.run(turn, move |state, turn| {
                    let mut provider = provider;
                    state.run_with(
                        turn,
                        ContextKey::Fixture(connection),
                        owned,
                        root,
                        &mut provider,
                    )
                })
            },
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let snapshot = service.session.snapshot().unwrap();
        if !snapshot.worker_running {
            assert!(
                snapshot.turns.last().unwrap().failure.is_none(),
                "{snapshot:?}"
            );
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    Arc::try_unwrap(replies).unwrap().into_inner().unwrap()
}
fn measure() -> Vec<Action> {
    vec![
        ("play_run", json!({"seconds":0.2,"profile":true}), true),
        ("profiler_read", json!({"frames":2}), true),
    ]
}
fn read(success: bool) -> Vec<Action> {
    vec![("profiler_read", json!({"frames":2}), success)]
}
#[test]
fn ordinary_editor_worker_advertises_real_profile_and_retains_it_between_turns() {
    let service = Service::new();
    let root = tempfile::tempdir().unwrap();
    let project = project();
    let bus = load(project.clone());
    let path = Some(root.path().to_path_buf());
    let replies = turn(&service, &bus, path.clone(), "account-a", measure());
    let original = &replies[1];
    assert_eq!(original["project_id"], project.id);
    assert_eq!(original["profile_id"], replies[0]["profile"]["id"]);
    assert_eq!(original["measurements"]["selected_frames"], 2);
    assert_eq!(original["measurements"]["failed_frames"], 0);
    assert_eq!(original["measurements"]["invalid_frames"], 0);
    for phase in ["player.frame", "script.tick"] {
        let row = original["measurements"]["zones"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == phase)
            .unwrap();
        assert_eq!(row["valid_frames"], 2);
        assert!(row["p95_wall_ns"].is_u64());
    }
    let mut actions = read(true);
    actions.push(("play_run", json!({"seconds":0,"profile":true}), false));
    actions.extend(read(true));
    let retained = turn(&service, &bus, path.clone(), "account-a", actions);
    assert_eq!(&retained[0], original);
    assert_eq!(&retained[2], original);
    let cleared = turn(
        &service,
        &bus,
        path,
        "account-a",
        vec![
            ("play_run", json!({"seconds":0.1}), true),
            ("profiler_read", json!({"frames":1}), false),
        ],
    );
    assert!(cleared[0].get("profile").is_none());
    let guard = bus.lock().unwrap();
    let crate::project::LoadState::Ready(bus) = &*guard else {
        panic!("project still ready")
    };
    assert_eq!(bus.project(), &project);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    service.shutdown();
}
#[test]
fn editor_owner_retires_profile_on_account_project_root_bus_and_shutdown_boundaries() {
    let service = Service::new();
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let mut bus = load(project());
    let mut root = Some(first_root.path().to_path_buf());
    let mut account = "account-a";
    let mut last_id = Value::Null;
    for boundary in 0..5 {
        let measured = turn(&service, &bus, root.clone(), account, measure());
        assert_ne!(measured[1]["profile_id"], last_id);
        last_id = measured[1]["profile_id"].clone();
        match boundary {
            0 => account = "account-b",
            1 => {
                *bus.lock().unwrap() =
                    crate::project::LoadState::Ready(Box::new(CommandBus::new(project()).unwrap()))
            }
            2 => root = Some(second_root.path().to_path_buf()),
            3 => {
                let project = bus.lock().unwrap().bus_mut().unwrap().project().clone();
                bus = load(project);
            }
            4 => root = None,
            _ => unreachable!(),
        }
        turn(&service, &bus, root.clone(), account, read(false));
    }
    service.shutdown();
    assert!(
        service
            .start(
                request(),
                || Ok(Connection::ApiKey),
                |_, _, _| panic!("closed owner must not dispatch")
            )
            .is_err()
    );
}
