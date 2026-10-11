use incant_agent::provider::{Completion, Provider};
use incant_agent::{Agent, AgentError, ApprovalMode, Budget, NoViewport, Usage, tools};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Project, ScriptSource, new_id};
use serde_json::{Value, json};
use std::sync::atomic::AtomicBool;

struct ScriptPatch(Option<Value>);

impl Provider for ScriptPatch {
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

fn source() -> ScriptSource {
    ScriptSource {
        id: new_id(),
        name: "Movement".into(),
        path: "scripts/movement.ts".into(),
    }
}

#[test]
fn registration_and_removal_use_the_existing_patch_approval_boundary() {
    for approval in [
        ApprovalMode::Destructive,
        ApprovalMode::Always,
        ApprovalMode::Auto,
    ] {
        for allow in [false, true] {
            for remove in [false, true] {
                let mut bus = CommandBus::new(Project::empty("Script tools")).unwrap();
                let script = source();
                if remove {
                    bus.execute(
                        vec![Command::UpsertScript {
                            script: script.clone(),
                        }],
                        Actor::user("author"),
                        "Register",
                        Some(0),
                    )
                    .unwrap();
                }
                let before = bus.project().clone();
                let revision = bus.revision();
                let command = if remove {
                    Command::RemoveScript {
                        script_id: script.id.clone(),
                    }
                } else {
                    Command::UpsertScript {
                        script: script.clone(),
                    }
                };
                let arguments = json!({"ops":[command],"description":"Edit script registration","expected_revision":revision});
                let mut provider = ScriptPatch(Some(
                    json!({"type":"function_call","name":"doc_patch","call_id":"script-patch","arguments":arguments.to_string()}),
                ));
                let mut agent = Agent {
                    budget: Budget {
                        max_tokens: 100_000,
                        used_tokens: 0,
                    },
                    approval,
                    max_steps: 3,
                    max_output_tokens: 25_000,
                };
                let mut approvals = 0;
                let report = agent
                    .run(
                        &mut provider,
                        &mut bus,
                        &mut NoViewport,
                        "Edit script registration",
                        "script-conversation",
                        &AtomicBool::new(false),
                        &mut |name, args| {
                            assert_eq!(name, "doc_patch");
                            assert_eq!(args, &arguments);
                            approvals += 1;
                            allow
                        },
                        &mut |_| {},
                    )
                    .unwrap();
                let auto = matches!(approval, ApprovalMode::Auto);
                assert_eq!(approvals, usize::from(!auto));
                assert_eq!(report.tool_calls[0].succeeded, allow || auto);
                if allow || auto {
                    assert_eq!(bus.revision(), revision + 1);
                    assert_eq!(bus.project().scripts.contains_key(&script.id), !remove);
                    assert_eq!(
                        report.transaction_ids,
                        vec![bus.history().last().unwrap().id.clone()]
                    );
                    assert_eq!(
                        bus.history().last().unwrap().actor,
                        Actor::agent("fixture-model", "script-conversation")
                    );
                    bus.undo().unwrap();
                    assert_eq!(bus.project(), &before);
                } else {
                    assert_eq!(bus.project(), &before);
                    assert_eq!(bus.revision(), revision);
                    assert_eq!(bus.history().len(), revision as usize);
                    assert!(report.transaction_ids.is_empty());
                }
            }
        }
    }
}

#[test]
fn script_metadata_is_exposed_by_the_shared_typed_patch_schema() {
    let tool = tools()
        .into_iter()
        .find(|tool| tool["name"] == "doc_patch")
        .unwrap();
    let definitions = &tool["parameters"]["$defs"];
    let commands = definitions["Command"]["oneOf"].as_array().unwrap();
    let upsert = commands
        .iter()
        .find(|command| command["properties"]["op"]["const"] == "upsert_script")
        .unwrap();
    assert_eq!(
        upsert["properties"]["script"]["$ref"],
        "#/$defs/ScriptSource"
    );
    assert_eq!(definitions["ScriptSource"]["additionalProperties"], false);
    assert!(
        commands
            .iter()
            .any(|command| command["properties"]["op"]["const"] == "remove_script")
    );
    assert!(serde_json::from_value::<Command>(json!({"op":"upsert_script","script":{"id":new_id(),"name":"Move","path":"scripts/move.ts","content":"arbitrary source"}})).is_err());
}
