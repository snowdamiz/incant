use super::*;
use incant_agent::provider::Provider;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Suite {
    version: u32,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    prompt: String,
    initial: Project,
    expected: Project,
    #[serde(default)]
    max_transactions: Option<usize>,
}

fn normalized(mut project: Project) -> Project {
    for scene in project.scenes.values_mut() {
        for entity in scene.entities.values_mut() {
            entity.provenance = None;
        }
    }
    project
}
fn load(path: &Path) -> Result<Suite> {
    let suite: Suite = serde_json::from_str(&fs::read_to_string(path)?)?;
    if suite.version != 1 || suite.cases.len() != 20 {
        return Err("Phase 0 suite must contain exactly twenty version-1 cases".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    for case in &suite.cases {
        case.initial.validate()?;
        case.expected.validate()?;
        if !ids.insert(&case.id) || case.prompt.is_empty() || case.initial == case.expected {
            return Err("duplicate, empty, or no-op eval case".into());
        }
    }
    Ok(suite)
}
pub fn check(path: &Path) -> Result<()> {
    let suite = load(path)?;
    print(json!({"validated_cases":suite.cases.len(),"live_inference":false,"gate_passed":false}))
}
pub fn run(path: &Path, output: &Path, model: String, max_tokens: u64, ci: bool) -> Result<()> {
    let suite = load(path)?;
    let mut provider = if ci {
        OpenAiProvider::new(
            std::env::var("INCANT_EVAL_OPENAI_API_KEY")
                .map_err(|_| "dedicated INCANT_EVAL_OPENAI_API_KEY required for CI")?,
            model.clone(),
        )?
    } else {
        super::provider(model.clone())?
    };
    provider.models()?;
    let mut viewport =
        GpuPerception(incant_render::Renderer::headless().map_err(|e| e.to_string())?);
    let mut results = vec![];
    let mut passed = 0;
    for case in suite.cases {
        eprintln!("Running {} with {}", case.id, provider.model());
        let mut bus = CommandBus::new(case.initial.clone())?;
        let mut agent = Agent {
            budget: Budget {
                max_tokens,
                used_tokens: 0,
            },
            approval: ApprovalMode::Auto,
            max_steps: 30,
        };
        let started = Instant::now();
        let result = agent.run(
            &mut provider,
            &mut bus,
            &mut viewport,
            &case.prompt,
            &new_id(),
            &AtomicBool::new(false),
            &mut |_, _| true,
            &mut |_| {},
        );
        let exact_state = normalized(bus.project().clone()) == normalized(case.expected);
        let all_tools = result.as_ref().is_ok_and(|r| {
            ["doc_query", "doc_patch", "view_screenshot"]
                .iter()
                .all(|name| r.tool_calls.iter().any(|t| t.name == *name && t.succeeded))
        });
        let provenance = !bus.history().is_empty()
            && bus.history().iter().all(|t| {
                t.actor.origin == incant_doc::Origin::Agent
                    && t.actor.model.as_deref() == Some(model.as_str())
            });
        let actual = bus.project().clone();
        let transaction_limit = case
            .max_transactions
            .is_none_or(|limit| bus.history().len() <= limit);
        let mut undo_ok = true;
        while !bus.history().is_empty() {
            if bus.undo().is_err() {
                undo_ok = false;
                break;
            }
        }
        undo_ok &= bus.project() == &case.initial;
        let ok = result.is_ok()
            && exact_state
            && all_tools
            && provenance
            && undo_ok
            && transaction_limit;
        passed += usize::from(ok);
        let (report, error) = match result {
            Ok(r) => (json!(r), Value::Null),
            Err(e) => (Value::Null, json!(e.to_string())),
        };
        results.push(json!({"case":case.id,"passed":ok,"exact_state":exact_state,"three_tools":all_tools,"provenance":provenance,"undo":undo_ok,"transaction_limit":transaction_limit,"elapsed_ms":started.elapsed().as_millis(),"used_tokens":agent.budget.used_tokens,"report":report,"error":error,"actual":actual}));
        // Persist after each case; interrupted runs cannot masquerade as 20/20.
        save(
            output,
            &(serde_json::to_string_pretty(
                &json!({"version":1,"live_inference":true,"model":model,"completed":results.len(),"passed":passed,"required":14,"gate_passed":results.len()==20 && passed>=14,"cases":results}),
            )? + "\n"),
        )?;
    }
    print(json!({"output":output,"passed":passed,"total":20,"gate_passed":passed>=14}))?;
    if passed < 14 {
        return Err("live evaluation did not reach 14 of 20".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corpus_has_twenty_valid_nontrivial_independent_cases() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../evals/phase0/tasks.json");
        let suite = load(&path).unwrap();
        for case in suite.cases {
            assert_ne!(normalized(case.initial), normalized(case.expected.clone()));
            let mut wrong = case.expected.clone();
            wrong.name.push_str("unrequested");
            assert_ne!(normalized(wrong), normalized(case.expected));
        }
    }
}
