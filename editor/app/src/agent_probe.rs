//! Opt-in live-account acceptance through the real native command handlers.
//! Only a disposable fixture is edited. Receipts contain fixed diagnostics and
//! numerical usage, never account metadata, provider responses or credentials.
use crate::{Editor, agent, project};
use incant_agent::session::{ApprovalChoice, ApprovalId, TurnRequest};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;
const CAMERA: &str = "00000000000000000000000003";

pub struct Options {
    output: PathBuf,
    model: String,
    restore_only: bool,
}
impl Options {
    pub fn from_args() -> Option<Self> {
        let mut args = std::env::args_os().skip(2);
        let flag = args.next()?;
        let restore_only = match flag.to_str()? {
            "--agent-probe" => false,
            "--agent-probe-restore" => true,
            _ => return None,
        };
        Some(Self {
            output: args.next().expect("agent probe output").into(),
            model: args
                .next()
                .expect("agent probe model")
                .into_string()
                .expect("UTF-8 model"),
            restore_only,
        })
    }
}
fn require(ok: bool, error: &'static str) -> Result<(), &'static str> {
    ok.then_some(()).ok_or(error)
}
fn read(app: &tauri::AppHandle) -> Result<Value, &'static str> {
    serde_json::to_value(
        tauri::async_runtime::block_on(agent::engine_agent_read(app.state(), None))
            .map_err(|_| "agent read failed")?,
    )
    .map_err(|_| "agent read serialization failed")
}
fn document(editor: &Editor) -> Result<(Value, u64, usize), &'static str> {
    let state = editor.bus.lock().map_err(|_| "project lock failed")?;
    let project::LoadState::Ready(bus) = &*state else {
        return Err("project not ready");
    };
    Ok((json!(bus.project()), bus.revision(), bus.history().len()))
}
fn journal_state(editor: &Editor) -> Result<Value, &'static str> {
    let state = editor.bus.lock().map_err(|_| "project lock failed")?;
    let project::LoadState::Ready(bus) = &*state else {
        return Err("project not ready");
    };
    Ok(
        json!({"revision":bus.revision(),"applied":bus.history().len(),"redo":bus.redo_history().count(),
        "transaction_ids":bus.history().iter().chain(bus.redo_history()).map(|tx|tx.id.clone()).collect::<Vec<_>>()}),
    )
}

/// An approval here grants only the exact disposable-fixture operation.
fn allowed(stage: usize, tool: &str, args: &Value, expected: &str) -> bool {
    match tool {
        "doc_query" => {
            args.as_object().is_some_and(|v| v.len() == 1)
                && matches!(
                    args["path"].as_str(),
                    Some("" | "/memory" | "/memory/live_probe")
                )
        }
        "doc_patch" if stage < 2 => {
            args["ops"] == json!([{"op":"set_memory", "section":"live_probe", "text":expected}])
        }
        "play_run" if stage == 2 => {
            args.as_object().is_some_and(|v| {
                v.iter().all(|(key, value)| {
                    key == "seconds"
                        || (key == "camera" && value == CAMERA)
                        || (matches!(
                            key.as_str(),
                            "scene" | "script_sha256" | "inputs_script" | "observe"
                        ) && value.is_null())
                })
            }) && args["seconds"] == 0.2
                && args["camera"] == CAMERA
        }
        "play_state_query" if stage == 3 => args == &json!({"path":"/tick"}),
        _ => false,
    }
}

fn turn(
    app: &tauri::AppHandle,
    options: &Options,
    stage: usize,
    prompt: String,
    expected: &str,
) -> Result<Value, &'static str> {
    let id = tauri::async_runtime::block_on(agent::engine_agent_start(
        app.state(),
        TurnRequest {
            prompt,
            model: options.model.clone(),
            approval: ApprovalChoice::Always,
            view_context: None,
        },
    ))
    .map_err(|_| "agent start failed")?;
    let result = (|| {
        let deadline = Instant::now() + Duration::from_secs(180);
        let mut decisions = 0;
        let mut last_approval = None;
        let mut observed_stream = false;
        loop {
            require(Instant::now() < deadline, "live turn deadline exceeded")?;
            let snapshot = read(app)?;
            let session = &snapshot["session"];
            let current = session["turns"]
                .as_array()
                .and_then(|v| v.last())
                .ok_or("turn absent")?;
            require(current["id"] == id.0, "turn identity changed")?;
            if current["text"].as_str().is_some_and(|v| !v.is_empty())
                && session["worker_running"] == true
            {
                observed_stream = true;
            }
            if let Some(approval) = current["pending"].as_object() {
                let sequence = approval["id"].as_u64().ok_or("approval identity absent")?;
                if last_approval != Some(sequence) {
                    require(decisions < 8, "probe approval count exceeded")?;
                    let tool = approval["tool"].as_str().ok_or("approval tool absent")?;
                    require(
                        allowed(stage, tool, &approval["arguments"], expected),
                        "provider requested an out-of-scope operation",
                    )?;
                    agent::engine_agent_decide(app.state(), id, ApprovalId(sequence), true)
                        .map_err(|_| "approval failed")?;
                    last_approval = Some(sequence);
                    decisions += 1;
                }
            }
            if session["worker_running"] == false {
                require(
                    current["status"] == "completed",
                    "live turn did not complete",
                )?;
                require(
                    current["accounting_uncertain"] == false
                        && session["accounting_uncertain"] == false,
                    "live usage uncertain",
                )?;
                require(
                    current["charged_tokens"].as_u64().is_some_and(|n| n > 0),
                    "measured live usage absent",
                )?;
                let calls = current["report"]["tool_calls"]
                    .as_array()
                    .ok_or("tool report absent")?;
                require(
                    calls.iter().all(|v| v["succeeded"] == true),
                    "a live tool failed",
                )?;
                let required = match stage {
                    0 | 1 => "doc_patch",
                    2 => "play_run",
                    _ => "play_state_query",
                };
                require(
                    calls.iter().any(|v| v["name"] == required),
                    "required live tool was not called",
                )?;
                require(decisions > 0, "no approval was observed")?;
                return Ok(
                    json!({"turn":id.0,"completed":true,"observed_stream_before_completion":observed_stream,
                    "approval_count":decisions,"report":current["report"],"charged_tokens":current["charged_tokens"],
                    "used_tokens":session["budget"]["used_tokens"]}),
                );
            }
            thread::sleep(Duration::from_millis(20));
        }
    })();
    if result.is_err() {
        let _ = agent::engine_agent_cancel(app.state(), id);
    }
    result
}

fn run(app: &tauri::AppHandle, editor: &Editor, options: &Options) -> Result<Value, &'static str> {
    let deadline = Instant::now() + Duration::from_secs(45);
    while document(editor).is_err() || editor.provider.connection().is_none() {
        require(
            Instant::now() < deadline,
            "saved account or project did not become ready",
        )?;
        thread::sleep(Duration::from_millis(50));
    }
    let before = document(editor)?;
    require(
        before.0["name"] == "Native live agent acceptance",
        "probe refuses a non-fixture project",
    )?;
    require(
        before.0["memory"]["live_probe"].is_null(),
        "probe fixture is already modified",
    )?;
    if options.restore_only {
        let restored = read(app)?;
        require(
            restored["session"]["worker_running"] == false,
            "reopen unexpectedly started a worker",
        )?;
        return Ok(
            json!({"passed":true,"saved_account_restored":true,"provider_turns":0,
            "journal_after_undo":journal_state(editor)?,
            "budget_after_undo":restored["session"]["budget"],
            "accounting_uncertain":restored["session"]["accounting_uncertain"]}),
        );
    }
    let initial_budget = read(app)?["session"]["budget"].clone();
    let nonce = ulid::Ulid::new().to_string();
    let first = turn(
        app,
        options,
        0,
        format!(
            "This disposable integration fixture has no private content. Query the project revision, then use exactly one doc_patch with one set_memory op to set section live_probe to {nonce}. Make no other changes. After the tool succeeds, say Done."
        ),
        &nonce,
    )?;
    let after_first = document(editor)?;
    require(
        after_first.0["memory"]["live_probe"] == nonce && after_first.1 == before.1 + 1,
        "first atomic edit was not observed",
    )?;
    let next = format!("{nonce}-followup");
    let second = turn(app, options, 1, "Continue the preceding edit: append -followup to the memory value you just wrote in live_probe. Query the current revision and use one atomic doc_patch with one set_memory op. Make no other changes. After success, say Done.".into(), &next)?;
    let authored = document(editor)?;
    require(
        authored.0["memory"]["live_probe"] == next
            && authored.1 == before.1 + 2
            && authored.2 == before.2 + 2,
        "follow-up atomic edit was not observed",
    )?;
    let third = turn(
        app,
        options,
        2,
        format!(
            "Run the current project using exactly one play_run with seconds 0.2 and camera {CAMERA}; omit all other fields or use null. Do not edit anything and do not query the runtime yet. After success, say Done."
        ),
        "",
    )?;
    let fourth = turn(app, options, 3, "Use exactly one play_state_query with path /tick to inspect the native game left running by your preceding turn. Do not call play_run again and do not edit anything. After success, say Done.".into(), "")?;
    require(
        document(editor)? == authored,
        "native Play changed authoring state",
    )?;
    {
        let state = editor.bus.lock().map_err(|_| "project lock failed")?;
        let project::LoadState::Ready(bus) = &*state else {
            return Err("project unavailable");
        };
        for transaction in bus.history().iter().skip(before.2) {
            let actor = json!(transaction.actor);
            require(
                actor["origin"] == "agent"
                    && actor["conversation_id"]
                        .as_str()
                        .is_some_and(|v| v.parse::<ulid::Ulid>().is_ok()),
                "agent provenance missing",
            )?;
        }
    }
    pollster::block_on(crate::engine_history(app.state(), false))
        .map_err(|_| "first undo failed")?;
    require(
        document(editor)?.0["memory"]["live_probe"] == nonce,
        "follow-up undo differs",
    )?;
    pollster::block_on(crate::engine_history(app.state(), false))
        .map_err(|_| "second undo failed")?;
    require(
        document(editor)?.0 == before.0,
        "atomic undo did not restore the project",
    )?;
    require(
        [&first, &second, &third, &fourth]
            .iter()
            .any(|v| v["observed_stream_before_completion"] == true),
        "no live stream was observed",
    )?;
    let after_undo = read(app)?;
    let charged: u64 = [&first, &second, &third, &fourth]
        .iter()
        .map(|value| value["charged_tokens"].as_u64().unwrap_or(0))
        .sum();
    require(
        after_undo["session"]["budget"]["used_tokens"].as_u64()
            == initial_budget["used_tokens"]
                .as_u64()
                .and_then(|used| used.checked_add(charged)),
        "Undo changed consumption accounting",
    )?;
    Ok(
        json!({"passed":true,"platform":std::env::consts::OS,"turns":[first,second,third,fourth],
        "native_handler_calls":true,"saved_account_restored":true,"agent_provenance":true,"undo_restored_document":true,
        "native_play_retained_across_turns":true,"play_preserved_authoring":true,"journal_after_undo":journal_state(editor)?,
        "budget_before":initial_budget,"budget_after_undo":after_undo["session"]["budget"],
        "accounting_uncertain":after_undo["session"]["accounting_uncertain"],
        "limitations":["Local live account and numerical native handler acceptance; not rendered-pixel or WebView IPC dispatch evidence","Not a model quality evaluation, physical-device measurement or phase gate"]}),
    )
}
pub fn start(app: tauri::AppHandle, editor: Arc<Editor>, options: Options) {
    thread::spawn(move || {
        let result = (|| -> Result<(), &'static str> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&options.output)
                .map_err(|_| "receipt creation failed")?;
            let result = run(&app, &editor, &options);
            let receipt = match &result {
                Ok(value) => value.clone(),
                Err(error) => json!({"passed":false,"error":error}),
            };
            file.write_all(
                serde_json::to_string_pretty(&receipt)
                    .map_err(|_| "receipt encoding failed")?
                    .as_bytes(),
            )
            .map_err(|_| "receipt write failed")?;
            file.sync_all().map_err(|_| "receipt sync failed")?;
            result.map(|_| ())
        })();
        editor.agent.shutdown();
        app.exit(if result.is_ok() { 0 } else { 1 });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approval_scope_refuses_extra_mutation_and_runtime_replacement() {
        let patch = json!({"expected_revision":0,"description":"test","ops":[{"op":"set_memory","section":"live_probe","text":"expected"}]});
        assert!(allowed(0, "doc_patch", &patch, "expected"));
        assert!(!allowed(0, "doc_patch", &patch, "different"));
        let mut extra = patch.clone();
        extra["ops"]
            .as_array_mut()
            .unwrap()
            .push(json!({"op":"set_memory","section":"other","text":"unexpected"}));
        assert!(!allowed(0, "doc_patch", &extra, "expected"));
        assert!(!allowed(2, "doc_patch", &patch, "expected"));
        assert!(allowed(
            2,
            "play_run",
            &json!({"seconds":0.2,"camera":CAMERA}),
            ""
        ));
        assert!(!allowed(
            2,
            "play_run",
            &json!({"seconds":0.2,"camera":CAMERA,"inputs_script":{}}),
            ""
        ));
        assert!(!allowed(2, "play_run", &json!({"seconds":0.2}), ""));
        assert!(!allowed(
            3,
            "play_run",
            &json!({"seconds":0.2,"camera":CAMERA}),
            ""
        ));
        assert!(allowed(3, "play_state_query", &json!({"path":"/tick"}), ""));
        assert!(!allowed(3, "view_screenshot", &json!({}), ""));
    }
}
