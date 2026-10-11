use super::{Controller, Error, Live, Observation, Result};
use crate::{Session, StartOptions};
use incant_agent::play::RunRequest;
use incant_doc::Project;
use incant_input::InputReplay;
use incant_observe::Plan;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

fn check_budget(cancel: &AtomicBool, started: Instant) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Interrupted);
    }
    if started.elapsed() >= Duration::from_secs(10) {
        return Err(Error::Deadline);
    }
    Ok(())
}
fn native(error: impl std::error::Error + 'static) -> Error {
    crate::SessionError::Native(Box::new(error)).into()
}

impl Controller {
    /// A new disposable candidate is published only after a completed run.
    /// Invalid input, interruption or runtime failure preserves any previous
    /// completed session; partial candidate state never escapes through queries.
    pub fn run(
        &mut self,
        project: &Project,
        revision: u64,
        root: &Path,
        request: RunRequest,
        cancel: &AtomicBool,
    ) -> Result<Value> {
        self.run_checked(project, revision, root, request, cancel, &mut || Ok(()))
    }

    /// The callback validates the original authoring snapshot after all source
    /// and asset reads have finished, before executing any candidate tick.
    #[allow(clippy::too_many_arguments)]
    pub fn run_checked(
        &mut self,
        project: &Project,
        revision: u64,
        root: &Path,
        request: RunRequest,
        cancel: &AtomicBool,
        validate: &mut dyn FnMut() -> Result<()>,
    ) -> Result<Value> {
        let started = Instant::now();
        check_budget(cancel, started)?;
        if serde_json::to_vec(&request)?.len() > 1024 * 1024 {
            return Err(Error::Invalid("play request exceeds 1 MiB"));
        }
        let rate = project.settings.tick_rate;
        let ticks = (request.seconds * f64::from(rate)).ceil();
        if !request.seconds.is_finite()
            || request.seconds <= 0.
            || request.seconds > 600.
            || !(1. ..=36000.).contains(&ticks)
        {
            return Err(Error::Invalid(
                "duration must be positive, at most 600 seconds and 36000 ticks",
            ));
        }
        let end = ticks as u64;
        let replay = request
            .inputs_script
            .map(|input| {
                let text = serde_json::to_string(&input)?;
                let replay = InputReplay::from_text(&text, 0, rate).map_err(native)?;
                if replay.end_tick() < end {
                    return Err(Error::Invalid("recording does not cover requested ticks"));
                }
                Ok(replay)
            })
            .transpose()?;
        let mut plan = request
            .observe
            .map(|request| Plan::parse(&serde_json::to_vec(&request)?, end).map_err(native))
            .transpose()?;
        check_budget(cancel, started)?;
        let session = Session::new(
            project,
            root,
            &StartOptions {
                scene: request.scene,
                camera: request.camera,
                script_sha256: request.script_sha256,
            },
            48000,
            self.fallback,
        )?;
        check_budget(cancel, started)?;
        validate()?;
        check_budget(cancel, started)?;
        let max_frames = (48000 / u64::from(session.world()?.tick_rate()) + 1) as usize;
        let mut candidate = Live {
            session,
            started_revision: revision,
            samples: vec![0.; max_frames * 2],
        };
        if let Some(plan) = &mut plan {
            plan.observe(candidate.session.world()?, candidate.session.input()?)
                .map_err(native)?;
        }
        for tick in 1..=end {
            check_budget(cancel, started)?;
            let frames = candidate.session.next_frames()?;
            let input = replay
                .as_ref()
                .map(|replay| replay.events_at(tick))
                .transpose()
                .map_err(native)?
                .unwrap_or(&[]);
            candidate
                .session
                .tick(input, &mut candidate.samples[..frames * 2])?;
            if let Some(plan) = &mut plan {
                plan.observe(candidate.session.world()?, candidate.session.input()?)
                    .map_err(native)?;
            }
        }
        check_budget(cancel, started)?;
        let observations = plan
            .map(|plan| {
                plan.finish(candidate.session.world().map_err(native)?)
                    .map_err(native)
            })
            .transpose()?;
        let state = candidate.state(
            &Observation {
                include_input: true,
                include_logs: true,
                ..Default::default()
            },
            Vec::new(),
        )?;
        let report = json!({
            "passed":observations.as_ref().is_none_or(|report|report.passed),
            "runtime":"native", "state":state, "observations":observations,
            "gameplay_images_available":false
        });
        // A failed assertion is a completed run with inspectable results, unlike
        // a failed simulation. No project transaction is created in either case.
        check_budget(cancel, started)?;
        self.live = Some(candidate);
        Ok(report)
    }

    pub fn state_query(&self, path: &str) -> Result<Value> {
        if path.is_empty() || path.len() > 512 || !path.starts_with('/') {
            return Err(Error::Invalid("use a bounded runtime JSON Pointer"));
        }
        let first = path[1..].split('/').next().unwrap_or_default();
        let mut observation = Observation::default();
        match first {
            "entities" => {
                let id = path
                    .split('/')
                    .nth(2)
                    .filter(|id| !id.is_empty())
                    .ok_or(Error::Invalid("select one entity by canonical ULID"))?;
                observation.entities.push(id.to_owned());
            }
            "input" => observation.include_input = true,
            "logs" => observation.include_logs = true,
            "runtime"
            | "playing"
            | "started_revision"
            | "scene"
            | "script_source_sha256"
            | "script_source_parsing"
            | "tick"
            | "tick_rate"
            | "entity_count"
            | "frames"
            | "cpu_render_input"
            | "gpu_submitted" => {}
            _ => return Err(Error::Invalid("unknown runtime observation path")),
        }
        let selected = observation.selected()?;
        let live = self
            .live
            .as_ref()
            .ok_or(Error::Invalid("no play session"))?;
        let state = live.state(&observation, selected)?;
        let value = state
            .pointer(path)
            .ok_or(Error::Invalid("runtime observation path does not exist"))?;
        Ok(
            json!({"runtime":"native", "tick":live.session.world()?.tick(), "started_revision":live.started_revision,
            "path":path, "untrusted_runtime_data":value}),
        )
    }
}
