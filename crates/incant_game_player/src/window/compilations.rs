//! Trusted, default-off native creation observation. Final-only I/O; no GPU waits.
#[cfg(test)]
mod tests;
use crate::session;
use incant_render::compilation::{self, Context, Observer, Phase};
use serde::Serialize;
use std::{
    cell::{Cell, RefCell},
    fs::{File, OpenOptions},
    io::{self, Write},
    rc::Rc,
};

const OUTPUT_LIMIT: usize = 4 * 1024 * 1024;
const PHASES: usize = 5;
#[derive(Clone, Copy)]
pub(super) enum Boundary {
    Focus,
    Resize,
    Suspend,
    Resume,
    Restart,
    Reconfigured,
    Recreated,
    Deferred,
    ReservationError,
}
const BOUNDARIES: usize = 9;
#[derive(Clone, Copy, Default, Serialize)]
struct Counts {
    started: u64,
    committed: u64,
    abandoned: u64,
}
#[derive(Default, Serialize)]
struct Stats {
    scopes: [Counts; PHASES],
    saturated: bool,
}
pub(super) struct Capture {
    pub renderer: compilation::Capture,
    file: Option<File>,
    requested: u64,
    capacity: usize,
    segment: Cell<u64>,
    boundary_overflow: Cell<bool>,
    boundaries: Cell<[u64; BOUNDARIES]>,
    surface_counters: Cell<Option<[u64; 3]>>,
    stats: Rc<RefCell<Stats>>,
}
impl Capture {
    pub fn new(path: &str, capacity: usize, requested: u64) -> session::Result<Self> {
        if path.is_empty()
            || path.len() > 4096
            || path.chars().any(char::is_control)
            || !(1..=36_000).contains(&requested)
        {
            return Err("invalid bounded compilation capture request".into());
        }
        let renderer = compilation::Capture::new(capacity)?;
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            renderer,
            file: Some(file),
            requested,
            capacity,
            segment: Cell::new(1),
            boundary_overflow: Cell::new(false),
            boundaries: Cell::new([0; BOUNDARIES]),
            surface_counters: Cell::new(None),
            stats: Rc::new(RefCell::new(Stats::default())),
        })
    }
    pub fn boundary(&self, reason: Boundary) {
        let mut counts = self.boundaries.get();
        let index = reason as usize;
        match counts[index].checked_add(1) {
            Some(next) => counts[index] = next,
            None => self.boundary_overflow.set(true),
        }
        self.boundaries.set(counts);
        match self.segment.get().checked_add(1) {
            Some(next) => self.segment.set(next),
            None => self.boundary_overflow.set(true),
        }
    }
    // Observe the unchanged reserve result. Context already stamped on a call
    // remains its entry identity; these transitions fence subsequent scopes.
    pub fn reservation(&self, before: [u64; 3], after: [u64; 3], ready: Option<bool>) {
        self.surface_counters.set(Some(after));
        if before[0] != after[0] {
            self.boundary(Boundary::Reconfigured);
        }
        if before[1] != after[1] {
            self.boundary(Boundary::Recreated);
        }
        match ready {
            Some(false) => self.boundary(Boundary::Deferred),
            None => self.boundary(Boundary::ReservationError),
            Some(true) => (),
        }
    }
    pub fn scope(
        &self,
        phase: Phase,
        generation: u64,
        surface: u64,
        step: u64,
    ) -> session::Result<Scope> {
        if self.boundary_overflow.get() {
            return Err("compilation segment identity exhausted".into());
        }
        let observer = self
            .renderer
            .observer()
            .ok_or("compilation observer not attached")?;
        let scope = observer
            .scope(Context {
                session_generation: generation,
                surface_generation: surface,
                segment: self.segment.get(),
                host_step: step,
                phase,
            })
            .map_err(|e| format!("compilation scope refused: {e:?}"))?;
        let index = phase_index(phase);
        let mut stats = self.stats.borrow_mut();
        let Stats { scopes, saturated } = &mut *stats;
        bump(&mut scopes[index].started, saturated);
        Ok(Scope {
            native: Some(scope),
            stats: Rc::clone(&self.stats),
            index,
            committed: false,
        })
    }
    pub fn finish(
        &mut self,
        steps: u64,
        generation: u64,
        code: i32,
        result: &session::Result<serde_json::Value>,
    ) -> session::Result<()> {
        self.write(serde_json::json!({"stage":"window_finalized","accepted_host_steps":steps,
            "session_generation":generation,"event_loop_code":code,"returned_report":result.is_ok(),
            "window_error":result.as_ref().err().map(|e|e.to_string().chars().take(4096).collect::<String>())}))
    }
    fn write(&mut self, finalization: serde_json::Value) -> session::Result<()> {
        let Some(file) = self.file.take() else {
            return Ok(());
        };
        let snapshot = self.renderer.observer().map(|o| snapshot_json(&o));
        let stats = self.stats.borrow();
        let mut writer = Limited {
            file,
            remaining: OUTPUT_LIMIT,
        };
        serde_json::to_writer(
            &mut writer,
            &serde_json::json!({
                "format":"incant-native-compilation","version":1,"diagnostic_only":true,
                "shader_stall_gate":false,"display_gate":false,"final_process_exit_status":null,
                "source_identity":"requires external executable/source/package binding",
                "requested_host_steps":self.requested,"capacity":self.capacity,
                "limits":{"rows":compilation::MAX_SAMPLES,"row_bytes_max":96,"output_bytes":OUTPUT_LIMIT},
                "snapshot_memory_additional":true,"observer_overhead":true,"gpu_waits_added":false,
                "scope_phase_order":["Unscoped","Startup","Catalog","Background","Gameplay"],
                "host_scopes":&*stats,"segment":self.segment.get(),"segment_overflow":self.boundary_overflow.get(),
                "boundary_order":["Focus","Resize","Suspend","Resume","Restart","Reconfigured","Recreated","Deferred","ReservationError"],"boundaries":self.boundaries.get(),
                "surface_counter_order":["reconfigurations","recreations","skipped"],"last_reservation_counters":self.surface_counters.get(),
                "scope_identity":"entry session/surface/segment/requested host step; not simulation tick",
                "scope_limits":"catalog and game startup/refresh/tick work only; other calls remain explicitly unscoped; a mid-scope surface transition fences only later scopes; scope commits do not prove drawable presentation",
                "snapshot":snapshot,"finalization":finalization
            }),
        )?;
        writer.file.flush()?;
        writer.file.sync_all()?;
        Ok(())
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        if self.file.is_some()
            && let Err(error) =
                self.write(serde_json::json!({"stage":"abandoned_before_finalization"}))
        {
            eprintln!("compilation sidecar failed: {error}");
        }
    }
}
pub(super) struct Scope {
    native: Option<compilation::Scope>,
    stats: Rc<RefCell<Stats>>,
    index: usize,
    committed: bool,
}
impl Scope {
    pub fn commit(&mut self) {
        self.committed = true;
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        drop(self.native.take());
        let mut stats = self.stats.borrow_mut();
        let Stats { scopes, saturated } = &mut *stats;
        bump(
            if self.committed {
                &mut scopes[self.index].committed
            } else {
                &mut scopes[self.index].abandoned
            },
            saturated,
        );
    }
}
pub(super) fn enter(
    capture: &Option<Capture>,
    phase: Phase,
    generation: u64,
    surface: u64,
    step: u64,
) -> session::Result<Option<Scope>> {
    capture
        .as_ref()
        .map(|c| c.scope(phase, generation, surface, step))
        .transpose()
}
// A diagnostic write error must not replace an existing gameplay/window error.
pub(super) fn preserve_result<T>(
    result: session::Result<T>,
    observed: session::Result<()>,
) -> session::Result<T> {
    match (result, observed) {
        (Err(original), Err(sidecar)) => {
            eprintln!("compilation sidecar also failed: {sidecar}");
            Err(original)
        }
        (Err(original), Ok(())) => Err(original),
        (Ok(_), Err(sidecar)) => Err(sidecar),
        (Ok(value), Ok(())) => Ok(value),
    }
}
pub(super) fn commit(scope: &mut Option<Scope>) {
    if let Some(scope) = scope {
        scope.commit();
    }
}
fn phase_index(phase: Phase) -> usize {
    match phase {
        Phase::Unscoped => 0,
        Phase::Startup => 1,
        Phase::Catalog => 2,
        Phase::Background => 3,
        Phase::Gameplay => 4,
    }
}
fn bump(value: &mut u64, saturated: &mut bool) {
    match value.checked_add(1) {
        Some(next) => *value = next,
        None => *saturated = true,
    }
}
fn counts(values: &[compilation::Counts; compilation::KIND_COUNT]) -> Vec<serde_json::Value> {
    values.iter().map(|c|serde_json::json!({"attempts":c.attempts,"successes":c.successes,"errors":c.errors,"unwound":c.unwound})).collect()
}
fn snapshot_json(observer: &Observer) -> serde_json::Value {
    let s = observer.snapshot();
    let rows: Vec<_> = s
        .rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "sequence":r.sequence,"session_generation":r.context.session_generation,
                "surface_generation":r.context.surface_generation,"segment":r.context.segment,
                "requested_host_step":r.context.host_step,"phase":format!("{:?}",r.context.phase),
                "begin_ns":r.begin_ns,"elapsed_ns":r.elapsed_ns,"kind":format!("{:?}",r.kind),
                "stage":format!("{:?}",r.stage),"outcome":format!("{:?}",r.outcome),
                "error_domain":format!("{:?}",r.error_domain),"error_code":r.error_code
            })
        })
        .collect();
    serde_json::json!({"device":s.device,"rows":rows,"counts":counts(&s.counts),"unbound":counts(&s.unbound),
        "kind_order":["NagaLibrary","MslLibrary","BinaryLibrary","RenderPipeline","MeshPipeline","ComputePipeline"],
        "overflow":s.overflow,"saturated":s.saturated,"capture_started":s.capture_started,
        "scope_active":s.scope_active,"snapshot_ns":s.snapshot_ns,
        "clock_anomalies":{"entry":s.clock_anomalies.entry,"completion":s.clock_anomalies.completion,"snapshot":s.clock_anomalies.snapshot}})
}
struct Limited {
    file: File,
    remaining: usize,
}
impl Write for Limited {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() > self.remaining {
            return Err(io::Error::other("compilation output limit"));
        }
        let n = self.file.write(buf)?;
        self.remaining -= n;
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
