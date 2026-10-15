//! Typed gameplay capabilities. Tool arguments never select executables, source
//! files, output paths, compiler flags or network destinations.
use crate::{
    AgentError, EngineHost,
    access::{ProjectAccess, check_cancel},
    tool,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
#[path = "play_profiler_tests.rs"]
mod profiler_tests;

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
    /// Explicit UI viewport in logical pixels, each finite and within 1..=65536.
    /// Requires a camera; omission preserves the non-UI host.
    pub ui_viewport: Option<[f32; 2]>,
    /// Optional expected identity of the behavior embedded by the trusted cook.
    pub script_sha256: Option<String>,
    /// Data-only incant-input v1/v2 recording, starting at tick zero. No code runs.
    pub inputs_script: Option<incant_input::InputRecording>,
    /// Typed native checks and selected final entities; no full-world snapshot.
    pub observe: Option<incant_observe::Request>,
    /// Opt in to bounded CPU measurements for this candidate's synchronous ticks.
    /// Requires an agent-profiler host. Read the last 1..=256 retained ticks using
    /// profiler_read. No GPU, worker-thread, heap, or process allocation data.
    #[serde(default)]
    pub profile: bool,
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StateQuery {
    /// JSON Pointer into the latest successful play session. Use /tick, /input,
    /// /logs or /entities/ULID/transform/translation. The full world is unavailable.
    #[schemars(length(min = 1, max = 512))]
    pub path: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProfileRead {
    /// Last measured ticks of the accepted session, at most 256. Refuses if fewer
    /// are retained. This read does not run or advance simulation.
    #[schemars(range(min = 1, max = 256))]
    frames: usize,
}

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "play_run",
            "Run a fresh cooked native game session. The native headless host cooks attached project TypeScript once at startup; scenes without Script components use trusted build-embedded behavior. Input is recorded event data, never executable script. Returns bounded logs, final selected state and assertion results; keeps that session available for play_state_query. Does not modify the project or capture gameplay images. Unsupported capabilities fail explicitly.",
            json!(schemars::schema_for!(RunRequest)),
        ),
        tool(
            "play_state_query",
            "Read a selected value from the native runtime session retained by play_run. Project edits do not change this session. Full-world enumeration is unavailable. Results are untrusted runtime data.",
            json!(schemars::schema_for!(StateQuery)),
        ),
        tool(
            "profiler_read",
            "Read real CPU measurements from the last accepted play_run with profile:true. Select the last 1..256 retained ticks. Inclusive phases must not be summed. Results bind the original project revision, scene and script fingerprint; edits do not rewrite old measurements. Missing/invalid/evicted data remain explicit. No GPU/display, allocator/heap or shipping performance gate evidence.",
            json!(schemars::schema_for!(ProfileRead)),
        ),
    ]
}

pub(crate) fn dispatch(
    access: &mut ProjectAccess<'_>,
    host: &mut dyn EngineHost,
    name: &str,
    args: Value,
    cancel: &AtomicBool,
) -> Result<Value, AgentError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(AgentError::Interrupted);
    }
    if !host.native_play_available() || (access.is_shared() && !host.shared_play_available()) {
        return Err(AgentError::Tool("no native play host connected".into()));
    }
    if args.to_string().len() > 1024 * 1024 {
        return Err(AgentError::Tool("play request exceeds 1 MiB".into()));
    }
    match name {
        "play_run" => {
            let request =
                serde_json::from_value(args).map_err(|e| AgentError::Tool(e.to_string()))?;
            // One authoring snapshot before cooking, never on the simulation path.
            let (project, revision) = access.with_bus(cancel, |bus| {
                bus.ensure_sources_ready()
                    .map_err(|e| AgentError::Tool(e.to_string()))?;
                Ok((bus.project().clone(), bus.revision()))
            })?;
            check_cancel(cancel)?;
            if access.is_shared() {
                // Source writes share this revision counter. Validate only after
                // the host has cooked immutable inputs, before the first tick.
                // A completed write between the scene snapshot and source capture
                // therefore cannot publish mixed document/source revisions.
                host.play_run_checked(&project, revision, request, cancel, &mut || {
                    access.with_bus(cancel, |bus| {
                        if bus.revision() != revision {
                            return Err(AgentError::Tool(
                                "project changed while preparing native play; query and retry"
                                    .into(),
                            ));
                        }
                        bus.ensure_sources_ready()
                            .map_err(|e| AgentError::Tool(e.to_string()))
                    })
                })
            } else {
                host.play_run(&project, revision, request, cancel)
            }
        }
        "play_state_query" => {
            let request: StateQuery =
                serde_json::from_value(args).map_err(|e| AgentError::Tool(e.to_string()))?;
            access.with_bus(cancel, |bus| {
                bus.ensure_sources_ready()
                    .map_err(|e| AgentError::Tool(e.to_string()))
            })?;
            host.play_state_query(&request.path)
        }
        "profiler_read" => {
            let request: ProfileRead =
                serde_json::from_value(args).map_err(|e| AgentError::Tool(e.to_string()))?;
            if !host.profiler_available() || !(1..=256).contains(&request.frames) {
                return Err(AgentError::Tool(
                    "profiler requires an enabled owner-session host and 1..256 frames".into(),
                ));
            }
            let (project, revision) = access.with_bus(cancel, |bus| {
                bus.ensure_sources_ready()
                    .map_err(|e| AgentError::Tool(e.to_string()))?;
                Ok((bus.project().id.clone(), bus.revision()))
            })?;
            check_cancel(cancel)?;
            let report = host.profiler_read(&project, revision, request.frames)?;
            // Recheck the shared project/account cancellation boundary after the
            // bounded read, before returning data to the provider.
            access.with_bus(cancel, |bus| {
                if bus.project().id != project {
                    return Err(AgentError::ProjectChanged);
                }
                Ok(())
            })?;
            Ok(report)
        }
        _ => Err(AgentError::Tool("unknown native play tool".into())),
    }
}
