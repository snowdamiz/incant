//! Generic doc_patch must not gain an unapproved source-file mutation route.
use incant_agent::provider::{Completion, Provider};
use incant_agent::{Agent, AgentError, ApprovalMode, Budget, NoViewport, Usage, tools};
use incant_cmd::{Actor, CommandBus};
use incant_doc::{Project, RustModuleSource, new_id};
use serde_json::{Value, json};
use std::{fs, sync::atomic::AtomicBool};

struct Patch(Option<Value>);
impl Provider for Patch {
    fn model(&self) -> &str {
        "fixture-model"
    }
    fn complete(
        &mut self,
        _: &[Value],
        _: &[Value],
        _: u64,
        _: &mut dyn FnMut(&str),
    ) -> Result<Completion, AgentError> {
        Ok(Completion {
            output: self.0.take().into_iter().collect(),
            usage: Usage {
                input_tokens: 20,
                output_tokens: 10,
            },
        })
    }
}
#[test]
fn rust_registration_write_and_removal_obey_generic_patch_approval() {
    for allow in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("movement")).unwrap();
        let module = RustModuleSource {
            id: new_id(),
            name: "Movement".into(),
            path: "movement/main.rs".into(),
        };
        let initial = Project::empty("Module tools");
        let mut bus = CommandBus::persistent_with_sources(
            dir.path().join("journal"),
            initial.clone(),
            dir.path(),
        )
        .unwrap();
        for remove in [false, true] {
            if remove && !allow {
                break;
            }
            let before = bus.project().clone();
            let revision = bus.revision();
            let ops = if remove {
                json!([{"op":"remove_module_source","module_id":module.id}])
            } else {
                json!([{"op":"upsert_module_source","module":module},
                    {"op":"write_module_source","module_id":module.id,"expected_source":{"state":"missing"},"source":"pub fn update() {}"}])
            };
            let args = json!({"ops":ops,"description":"Author Rust","expected_revision":revision});
            let mut provider = Patch(Some(
                json!({"type":"function_call","name":"doc_patch","call_id":"rust-patch","arguments":args.to_string()}),
            ));
            let mut agent = Agent {
                budget: Budget {
                    max_tokens: 100_000,
                    used_tokens: 0,
                },
                approval: ApprovalMode::Destructive,
                max_steps: 3,
                max_output_tokens: 25_000,
            };
            let mut approvals = 0;
            let report = agent
                .run(
                    &mut provider,
                    &mut bus,
                    &mut NoViewport,
                    "Author Rust",
                    "rust-turn",
                    &AtomicBool::new(false),
                    &mut |name, actual| {
                        assert_eq!(name, "doc_patch");
                        assert_eq!(actual, &args);
                        approvals += 1;
                        allow
                    },
                    &mut |_| {},
                )
                .unwrap();
            assert_eq!(approvals, 1);
            assert_eq!(report.tool_calls[0].succeeded, allow);
            if allow {
                assert_eq!(
                    bus.history().last().unwrap().actor,
                    Actor::agent("fixture-model", "rust-turn")
                );
                assert_eq!(
                    report.transaction_ids,
                    vec![bus.history().last().unwrap().id.clone()]
                );
                assert_eq!(
                    fs::read_to_string(dir.path().join(&module.path)).unwrap(),
                    "pub fn update() {}"
                );
                assert_eq!(
                    bus.project().module_sources.contains_key(&module.id),
                    !remove
                );
            } else {
                assert_eq!(bus.project(), &before);
                assert_eq!(bus.revision(), revision);
                assert!(bus.history().is_empty());
                assert!(report.transaction_ids.is_empty());
                assert!(!dir.path().join(&module.path).exists());
            }
        }
        if allow {
            bus.undo().unwrap();
            bus.undo().unwrap();
            assert_eq!(bus.project(), &initial);
            assert!(!dir.path().join(&module.path).exists());
        }
    }
}
#[test]
fn patch_schema_exposes_closed_registered_rust_commands_without_compiler_authority() {
    let patch = tools()
        .into_iter()
        .find(|tool| tool["name"] == "doc_patch")
        .unwrap();
    let defs = &patch["parameters"]["$defs"];
    let ops = defs["Command"]["oneOf"].as_array().unwrap();
    for op in [
        "upsert_module_source",
        "remove_module_source",
        "write_module_source",
    ] {
        let schema = ops
            .iter()
            .find(|entry| entry["properties"]["op"]["const"] == op)
            .unwrap();
        assert_eq!(schema["additionalProperties"], false);
    }
    assert_eq!(defs["RustModuleSource"]["additionalProperties"], false);
    assert!(serde_json::from_value::<incant_cmd::Command>(json!({"op":"write_module_source","module_id":new_id(),"source":"text","expected_source":{"state":"missing"},"shell":"cargo build"})).is_err());
}
