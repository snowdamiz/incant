//! Trusted host-only, default-off exact drawable capture. No frame-loop I/O/waits.
#[cfg(test)]
mod batch_tests;
mod boundaries;
#[cfg(test)]
mod boundary_tests;
mod model;
#[cfg(test)]
mod tests;
use super::Error;
use crate::session;
pub(super) use boundaries::AcquisitionOutcome;
use incant_render::wgpu;
pub(super) use model::{Boundary, Failure, Role, Status};
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    sync::Arc,
    time::{Duration, Instant},
};
use wgpu_hal::metal::presentation::{Collector, Context, Observation, Outcome};

pub(super) const MAX_SAMPLES: usize = 36_000;
const OUTPUT_LIMIT: usize = 64 * 1024 * 1024;
const DRAIN: Duration = Duration::from_secs(1);
pub(super) struct Capture {
    collector: Arc<Collector>,
    model: model::Model,
    output: Option<File>,
    capacity: usize,
    requested_steps: u64,
    clock_origin: Instant,
    wakes: boundaries::Wakes,
    pulse_wake: Option<boundaries::PulseWake>,
    batches: bool,
}
/// The scheduler's existing decision, observed without changing its clock.
pub(super) struct Schedule {
    pub pulse: Instant,
    pub deadline: Option<Instant>,
    pub due: usize,
    pub ordinal: usize,
    pub discarded_ticks: u128,
}
impl Capture {
    pub fn new(path: &str, capacity: usize, requested_steps: u64) -> session::Result<Self> {
        if !(1..=MAX_SAMPLES).contains(&capacity) || !(1..=36_000).contains(&requested_steps) {
            return Err("presentation capture bounds1..36000".into());
        }
        let collector = Collector::new(capacity).map_err(observation_error)?;
        let model = model::Model::new(capacity)?;
        // Only a trusted CLI argument supplies this path. Never replace a file,
        // follow a final symlink or obtain any path from project/package data.
        let output = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            collector,
            model,
            output: Some(output),
            capacity,
            requested_steps,
            clock_origin: Instant::now(),
            wakes: boundaries::Wakes::default(),
            pulse_wake: None,
            batches: false,
        })
    }
    pub fn enable_batches(&mut self, selected: bool) {
        self.batches = selected;
    }
    pub fn batch_begin(&mut self, due: usize) {
        self.model.batch_begin(due);
    }
    pub fn batch_accept(&mut self, step: u64, displayed: bool) {
        self.model.batch_accept(step, displayed);
    }
    pub fn check_surface(&self, surface: &wgpu::Surface<'_>) -> Result<(), Error> {
        let context = Context::new(1, 1, 0).map_err(observation_error)?;
        let observation =
            Observation::new(Arc::clone(&self.collector), context).map_err(observation_error)?;
        bind(surface, Some(observation))?;
        bind(surface, None)
    }
    pub fn begin(&mut self, role: Role, generation: u64, step: u64) {
        self.model
            .begin(role, generation, step, self.time(Instant::now()));
    }
    pub fn gameplay_begin(&mut self, generation: u64, step: u64, schedule: Schedule) {
        self.begin(Role::Gameplay, generation, step);
        let pulse_ns = self.time(schedule.pulse);
        self.model
            .wake(self.pulse_wake.filter(|wake| wake.pulse_ns == pulse_ns));
        self.model.schedule(model::Schedule {
            pulse_ns: self.time(schedule.pulse),
            deadline_ns: schedule.deadline.and_then(|t| self.time(t)),
            forced_resume: schedule.deadline.is_none(),
            due: u8::try_from(schedule.due).unwrap_or(u8::MAX),
            ordinal: u8::try_from(schedule.ordinal).unwrap_or(u8::MAX),
            discarded_ticks: u64::try_from(schedule.discarded_ticks).ok(),
        });
    }
    pub fn new_events(&mut self, cause: tao::event::StartCause, arrived: Instant) {
        use boundaries::WakeCause;
        use tao::event::StartCause;
        let (kind, requested) = match cause {
            StartCause::Init => (WakeCause::Init, None),
            StartCause::Poll => (WakeCause::Poll, None),
            StartCause::WaitCancelled {
                requested_resume, ..
            } => (WakeCause::WaitCancelled, requested_resume),
            StartCause::ResumeTimeReached {
                requested_resume, ..
            } => (WakeCause::ResumeTimeReached, Some(requested_resume)),
            _ => (WakeCause::Unknown, None),
        };
        self.wakes.event(
            kind,
            self.time(arrived),
            requested.is_some(),
            requested.and_then(|at| self.time(at)),
        );
    }
    pub fn pulse(&mut self, pulse: Instant) {
        self.pulse_wake = self.wakes.pulse(self.time(pulse));
    }
    pub fn acquisition_begin(&mut self) {
        self.model.acquisition_begin(self.time(Instant::now()));
    }
    pub fn acquisition_end(&mut self, outcome: AcquisitionOutcome) {
        self.model
            .acquisition_end(self.time(Instant::now()), outcome);
    }
    fn time(&self, instant: Instant) -> Option<u64> {
        instant
            .checked_duration_since(self.clock_origin)?
            .as_nanos()
            .try_into()
            .ok()
    }
    pub fn reserved(&mut self) {
        self.model.reserved(self.time(Instant::now()));
    }
    pub fn host_finished(&mut self) {
        self.model.host_finished(self.time(Instant::now()));
    }
    pub fn boundary(&mut self, reason: Boundary) {
        self.model.boundary(reason);
    }
    pub fn recreated(&mut self) {
        self.model.recreated();
    }
    pub fn failure(&mut self, failure: Failure) {
        self.model.failure(failure);
    }
    pub fn complete(&mut self, status: Status) {
        self.model.complete(status, self.time(Instant::now()));
    }
    pub fn bind(&mut self, surface: &wgpu::Surface<'_>) -> Result<(), Error> {
        let observation = self
            .model
            .context()
            .map(|(surface_generation, segment, step, attempt)| {
                let context =
                    Context::new(surface_generation, segment, step)?.with_attempt(attempt)?;
                Observation::new(Arc::clone(&self.collector), context)
            })
            .transpose()
            .map_err(observation_error)?;
        bind(surface, observation)
    }
    pub fn unbind(surface: &wgpu::Surface<'_>) -> Result<(), Error> {
        bind(surface, None)
    }
    pub fn finish(
        &mut self,
        host_steps: u64,
        generation: u64,
        code: i32,
        result: &session::Result<serde_json::Value>,
        unbind_error: Option<String>,
    ) -> session::Result<()> {
        // Called after the game has stopped and observation is unbound. No native
        // object is held by callbacks; the host alone keeps this surface alive.
        let start = Instant::now();
        let mut pending = self.collector.pending();
        while pending != 0 && start.elapsed() < DRAIN {
            std::thread::sleep(
                Duration::from_millis(10).min(DRAIN.saturating_sub(start.elapsed())),
            );
            pending = self.collector.pending();
        }
        let status = serde_json::json!({"stage":"window_finalized", "event_loop_code":code,
            "accepted_host_steps":host_steps,"session_generation":generation,
            "returned_report":result.is_ok(),"window_error":result.as_ref().err().map(|e|bounded_error(&e.to_string())),
            "recording_completed":result.as_ref().ok().and_then(|v|v.get("input_recording")).and_then(|v|v.get("completed")).cloned(),
            "assertions_passed":result.as_ref().ok().and_then(|v|v.get("observations")).and_then(|v|v.get("passed")).cloned(),
            "unbind_error":unbind_error,"drain_elapsed_seconds":start.elapsed().as_secs_f64(),
            "pending_at_last_drain_scan":pending});
        self.write(status)
    }
    fn write(&mut self, status: serde_json::Value) -> session::Result<()> {
        let Some(file) = self.output.take() else {
            return Ok(());
        };
        let snapshot = self.collector.snapshot();
        let coverage = snapshot.coverage();
        let rows: Vec<_> = snapshot
            .rows
            .iter()
            .map(|r| {
                let (outcome, seconds, invalid_bits) = match r.outcome {
                    Outcome::Acquired => ("acquired", None, None),
                    Outcome::Requested => ("requested_missing_callback", None, None),
                    Outcome::Presented(t) => ("presented", Some(t), None),
                    Outcome::ReportedZero => ("reported_zero", None, None),
                    Outcome::Invalid(t) => ("invalid", None, Some(t.to_bits())),
                    Outcome::Discarded => ("discarded", None, None),
                    Outcome::Abandoned => ("abandoned", None, None),
                };
                Row {
                    surface_generation: r.context.surface_generation,
                    segment: r.context.segment,
                    requested_host_step: r.context.host_step,
                    attempt: r.context.attempt,
                    drawable_id: r.drawable_id,
                    outcome,
                    presented_seconds: seconds,
                    invalid_bits,
                }
            })
            .collect();
        let mut writer = Limited {
            file,
            remaining: OUTPUT_LIMIT,
        };
        let mut report = serde_json::json!({
            "format":"incant-native-presentations","version":if self.batches { 3 } else { 2 },"diagnostic_only":true,
            "display_gate":false,"uncapped_gpu_throughput":false,"final_process_exit_status":null,
            "source_identity":"requires external executable/source/package binding",
            "per_present_registration_overhead":true,"snapshot_after_gameplay":true,
            "requested_host_steps":self.requested_steps,"capture_capacity":self.capacity,
            "limits":{"rows":MAX_SAMPLES,"row_bytes_max":64,"attempt_bytes_max":512,"segment_bytes_max":40,
                "segments_per_attempt_max":2,"drain_seconds":1,"output_bytes":OUTPUT_LIMIT},
            "host_timing_contract":{"version":2,"unit":"nanoseconds","clock":"Rust Instant relative to capture creation",
                "shared_epoch_with_metal":false,"changes_frame_admission":false,
                "pulse":"MainEventsCleared entry, before polling and eligibility work",
                "deadline":"existing fixed-step clock deadline before due calculation; absent for forced resume",
                "began":"attempt before surface reservation","reserved":"successful reservation including view creation",
                "completed":"existing host-commit or refresh/defer point; not GPU completion",
                "finished":"gameplay after observations and haptics; before restart replacement or event-loop cleanup",
                "missing":"unreached, invalid or unavailable; never zero-filled",
                "wake":"NewEvents arrival consumed once by MainEventsCleared; missing/overwritten/deadline mismatch is not a timer verdict",
                "acquisition":"existing get_current_texture call only, before unbind and view creation; not GPU completion"},
            "wake_tracking":self.wakes.counts,
            "coverage":{"rows":coverage.rows,"segments":coverage.segments,"acquired":coverage.acquired,
                "missing":coverage.requested,"presented":coverage.presented,"zero":coverage.reported_zero,
                "invalid":coverage.invalid,"discarded":coverage.discarded,"abandoned":coverage.abandoned,
                "overflow":snapshot.overflow,"protocol_errors":snapshot.protocol_errors},
            "boundary_bit_order":["start","focus","suspend","resume","resize","configure","recreate","retry","restart","role","inactive"],
            "failure_index_order":["timeout","occluded","outdated","lost","validation","unsupported_size","minimized","reservation","device"],
            "host":self.model,"rows":rows,"finalization":status
        });
        if self.batches {
            report["batch_contract"] = serde_json::json!({
                "version":1,"maximum_ticks":4,
                "drawable_context":"first requested step, immutable after acquisition",
                "displayed_state":"attempt.batch.displayed_host_step, same session generation",
                "coverage":"accepted_ticks contiguous from requested_host_step; fewer than planned allowed on restart or stop bound",
                "ordinary_analyzer_compatible":false
            });
        }
        serde_json::to_writer(&mut writer, &report)?;
        writer.file.flush()?;
        writer.file.sync_all()?;
        Ok(())
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        if self.output.is_some() {
            // Setup/early error path only: no claim that callbacks were drained.
            if let Err(error) = self
                .write(serde_json::json!({"stage":"abandoned_before_finalization","drained":false}))
            {
                eprintln!("presentation sidecar failed: {error}");
            }
        }
    }
}
#[derive(Serialize)]
struct Row {
    surface_generation: u64,
    segment: u64,
    requested_host_step: u64,
    attempt: u64,
    drawable_id: u64,
    outcome: &'static str,
    presented_seconds: Option<f64>,
    invalid_bits: Option<u64>,
}
fn observation_error(error: wgpu_hal::metal::presentation::Error) -> Error {
    Error::Presentation(format!("{error:?}"))
}
fn bind(surface: &wgpu::Surface<'_>, observation: Option<Observation>) -> Result<(), Error> {
    // SAFETY: scoped borrow of the existing surface only. No raw resource is
    // destroyed, retained or acquired; no layer scheduling property is changed.
    let hal = unsafe { surface.as_hal::<wgpu::hal::api::Metal>() }
        .ok_or_else(|| Error::Presentation("Metal backend required".into()))?;
    hal.set_presentation_observation(observation)
        .map_err(observation_error)
}
fn bounded_error(error: &str) -> String {
    error.chars().take(4096).collect()
}
struct Limited {
    file: File,
    remaining: usize,
}
impl Write for Limited {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(io::Error::other("presentation output exceeds64MiB"));
        }
        let written = self.file.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

pub(super) fn preserve_result(
    result: session::Result<serde_json::Value>,
    observed: session::Result<()>,
) -> session::Result<serde_json::Value> {
    match result {
        Err(error) => {
            if let Err(sidecar) = observed {
                eprintln!("presentation sidecar failed: {sidecar}");
            }
            Err(error)
        }
        Ok(report) => {
            observed?;
            Ok(report)
        }
    }
}
