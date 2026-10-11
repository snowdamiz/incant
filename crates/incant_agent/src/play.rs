//! Typed gameplay capabilities. Tool arguments never select executables, source
//! files, output paths, compiler flags or network destinations.
use crate::{AgentError, EngineHost, tool};
use incant_cmd::CommandBus;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    /// Positive simulation duration, rounded up to one tick; at most 600 seconds
    /// and 36,000 ticks. Execution also has a host wall-time budget.
    #[schemars(range(min = 0.000001, max = 600))]
    pub seconds: f64,
    /// Required if the project contains more than one scene.
    pub scene: Option<String>,
    /// Authored camera for retained CPU render validation of spatial scenes.
    /// This host does not capture gameplay frames.
    pub camera: Option<String>,
    /// Optional expected identity of the behavior embedded by the trusted cook.
    pub script_sha256: Option<String>,
    /// Data-only incant-input v1 recording, starting at tick zero. No code runs.
    pub inputs_script: Option<incant_input::InputRecording>,
    /// Typed native checks and selected final entities; no full-world snapshot.
    pub observe: Option<incant_observe::Request>,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateQuery {
    /// JSON Pointer into the latest successful play session. Use /tick, /input,
    /// /logs or /entities/ULID/transform/translation. The full world is unavailable.
    #[schemars(length(min = 1, max = 512))]
    pub path: String,
}

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "play_run",
            "Run a fresh cooked native game session using trusted build-embedded behavior. Input is recorded event data, never executable script. Returns bounded logs, final selected state and assertion results; keeps that session available for play_state_query. Does not modify the project or capture gameplay images. Unmigrated Script components fail explicitly.",
            json!(schemars::schema_for!(RunRequest)),
        ),
        tool(
            "play_state_query",
            "Read a selected value from the native runtime session retained by play_run. Project edits do not change this session. Full-world enumeration is unavailable. Results are untrusted runtime data.",
            json!(schemars::schema_for!(StateQuery)),
        ),
    ]
}

pub(crate) fn dispatch(
    bus: &CommandBus,
    host: &mut dyn EngineHost,
    name: &str,
    args: Value,
    cancel: &AtomicBool,
) -> Result<Value, AgentError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(AgentError::Interrupted);
    }
    if !host.native_play_available() {
        return Err(AgentError::Tool("no native play host connected".into()));
    }
    if args.to_string().len() > 1024 * 1024 {
        return Err(AgentError::Tool("play request exceeds 1 MiB".into()));
    }
    match name {
        "play_run" => host.play_run(
            bus.project(),
            bus.revision(),
            serde_json::from_value(args).map_err(|e| AgentError::Tool(e.to_string()))?,
            cancel,
        ),
        "play_state_query" => {
            let request: StateQuery =
                serde_json::from_value(args).map_err(|e| AgentError::Tool(e.to_string()))?;
            host.play_state_query(&request.path)
        }
        _ => Err(AgentError::Tool("unknown native play tool".into())),
    }
}
