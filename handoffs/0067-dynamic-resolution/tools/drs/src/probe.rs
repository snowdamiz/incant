//! Test-owned target provider at the host boundary, a bounded event log and a
//! completion witness.
//!
//! The witness is an extra `Queue::on_submitted_work_done` callback registered
//! inside `RenderFrame::present`, immediately before the consumer registers its
//! own callback for the same submission. wgpu fires both in registration order in
//! one dispatch batch, so the witness's phase and time bound when the host's
//! callback ran. It never touches the consumer or its mailbox.
//!
//! Explicitly identified injections (all off unless a case arms them):
//! - `fault`: refuse acquisition (1) or presentation (2), the existing test
//!   providers' failure modes;
//! - `acquire_poll`: one blocking `Device::poll(Wait)` inside `acquire`, which the
//!   consumer calls after it has taken any completed result: dispatch timing;
//! - `arm_backlog`: one test-owned buffer-copy submission inside `present`, so the
//!   consumer's next callback waits behind queued GPU work: queue backlog.
use crate::{FORMAT, Renderer, Result, RetainedTarget, Viewport, wgpu};
use incant_render_gpu_host::{RenderFrame, TargetProvider};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const BACKLOG_BYTES: u64 = 64 << 20;
pub const BACKLOG_COPIES: u32 = 48;
/// Explicit test-driven dispatch and readback phases.
pub const PHASE_TEST_POLL: u32 = 9001;
pub const PHASE_READBACK: u32 = 9002;
pub const PHASE_DROP: u32 = 9003;
pub const PHASE_PREPARE: u32 = 9004;
pub const PHASE_AFTER_FAILURE: u32 = 9005;
pub const PHASE_IDLE: u32 = 9000;
const MAX_EVENTS: usize = 4096;

#[derive(Clone, Debug)]
pub struct Event {
    pub at_ns: u64,
    pub kind: &'static str,
    pub probe: &'static str,
    pub frame: u64,
    pub phase: u32,
}

/// Monotonic event log shared by every probe of one process. Phase 10j+1 is
/// tick j before the consumer takes its completed result, 10j+2 from the
/// provider's acquire onwards, 10j+3 after the tick returned.
pub struct Log {
    epoch: Instant,
    phase: AtomicU32,
    tick: AtomicU64,
    events: Mutex<Vec<Event>>,
}
impl Log {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            epoch: Instant::now(),
            phase: AtomicU32::new(PHASE_IDLE),
            tick: AtomicU64::new(0),
            events: Mutex::new(Vec::with_capacity(MAX_EVENTS)),
        })
    }
    pub fn now_ns(&self) -> u64 {
        self.epoch.elapsed().as_nanos() as u64
    }
    pub fn phase(&self) -> u32 {
        self.phase.load(Ordering::SeqCst)
    }
    pub fn set_phase(&self, phase: u32) {
        self.phase.store(phase, Ordering::SeqCst);
    }
    pub fn record(&self, kind: &'static str, probe: &'static str, frame: u64) {
        let event = Event {
            at_ns: self.now_ns(),
            kind,
            probe,
            frame,
            phase: self.phase(),
        };
        let mut events = self.events.lock().expect("event log");
        assert!(events.len() < MAX_EVENTS, "event log bound");
        events.push(event);
    }
    pub fn begin_tick(&self, tick: u64) {
        self.tick.store(tick, Ordering::SeqCst);
        self.set_phase(10 * tick as u32 + 1);
        self.record("tick-call", "-", tick);
    }
    pub fn end_tick(&self, tick: u64) {
        self.record("tick-return", "-", tick);
        self.set_phase(10 * tick as u32 + 3);
    }
    fn enter_acquire(&self) {
        self.set_phase(10 * self.tick.load(Ordering::SeqCst) as u32 + 2);
    }
    pub fn events(&self) -> Vec<Event> {
        self.events.lock().expect("event log").clone()
    }
    pub fn json(&self) -> Value {
        Value::Array(
            self.events()
                .iter()
                .map(|e| json!([e.at_ns, e.kind, e.probe, e.frame, e.phase]))
                .collect(),
        )
    }
    /// Explicit test-driven dispatch: a blocking poll outside any tick.
    pub fn poll(&self, renderer: &Renderer, phase: u32) -> Result<()> {
        let previous = self.phase();
        self.set_phase(phase);
        self.record("test-poll-start", "-", 0);
        renderer.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })?;
        self.record("test-poll-end", "-", 0);
        self.set_phase(previous);
        Ok(())
    }
}

pub struct Probe {
    pub name: &'static str,
    pub renderer: Arc<Renderer>,
    pub log: Arc<Log>,
    texture: Mutex<wgpu::Texture>,
    viewport: Mutex<Option<Viewport>>,
    pub fault: AtomicU8,
    acquire_poll: AtomicBool,
    backlog_armed: AtomicBool,
    backlog: Mutex<Option<[wgpu::Buffer; 2]>>,
    pub acquires: AtomicU64,
    pub presents: AtomicU64,
}
pub fn target_texture(renderer: &Renderer, size: [u32; 2]) -> wgpu::Texture {
    renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("0067 physical output target"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}
impl Probe {
    pub fn new(
        name: &'static str,
        renderer: &Arc<Renderer>,
        log: &Arc<Log>,
        size: [u32; 2],
        viewport: Option<Viewport>,
    ) -> Arc<Self> {
        Arc::new(Self {
            name,
            texture: Mutex::new(target_texture(renderer, size)),
            renderer: Arc::clone(renderer),
            log: Arc::clone(log),
            viewport: Mutex::new(viewport),
            fault: AtomicU8::new(0),
            acquire_poll: AtomicBool::new(false),
            backlog_armed: AtomicBool::new(false),
            backlog: Mutex::new(None),
            acquires: AtomicU64::new(0),
            presents: AtomicU64::new(0),
        })
    }
    pub fn targets(self: &Arc<Self>) -> Targets {
        Targets(Arc::clone(self))
    }
    /// A replaced surface texture of a new physical size (a viewport resize).
    pub fn resize(&self, size: [u32; 2]) {
        *self.texture.lock().expect("target") = target_texture(&self.renderer, size);
    }
    pub fn set_viewport(&self, viewport: Option<Viewport>) {
        *self.viewport.lock().expect("viewport") = viewport;
    }
    pub fn size(&self) -> [u32; 2] {
        let t = self.texture.lock().expect("target");
        [t.width(), t.height()]
    }
    pub fn arm_acquire_poll(&self) {
        self.acquire_poll.store(true, Ordering::SeqCst);
    }
    pub fn arm_backlog(&self) {
        self.backlog_armed.store(true, Ordering::SeqCst);
    }
    /// Fill the target with one colour (staged write, no frame).
    pub fn fill(&self, rgba: [u8; 4]) {
        let texture = self.texture.lock().expect("target");
        let bytes: Vec<u8> = rgba
            .iter()
            .copied()
            .cycle()
            .take((texture.width() * texture.height() * 4) as usize)
            .collect();
        self.renderer.queue.write_texture(
            texture.as_image_copy(),
            &bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * 4),
                rows_per_image: Some(texture.height()),
            },
            texture.size(),
        );
    }
    /// Read the physical target. The blocking poll is a test readback phase.
    pub fn pixels(&self) -> Result<crate::pixels::Rgba> {
        let texture = self.texture.lock().expect("target").clone();
        let previous = self.log.phase();
        self.log.set_phase(PHASE_READBACK);
        let result = crate::pixels::read(&self.renderer, &texture);
        self.log.set_phase(previous);
        result
    }
    fn witness(&self, frame: u64) {
        let log = Arc::clone(&self.log);
        let name = self.name;
        self.log.record("witness-registered", name, frame);
        self.renderer
            .queue
            .on_submitted_work_done(move || log.record("fire", name, frame));
    }
    fn submit_backlog(&self, frame: u64) {
        let device = &self.renderer.device;
        let mut slot = self.backlog.lock().expect("backlog");
        let [source, destination] = slot.get_or_insert_with(|| {
            [wgpu::BufferUsages::COPY_SRC, wgpu::BufferUsages::COPY_DST].map(|usage| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("0067 injected queue backlog"),
                    size: BACKLOG_BYTES,
                    usage,
                    mapped_at_creation: false,
                })
            })
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        for _ in 0..BACKLOG_COPIES {
            encoder.copy_buffer_to_buffer(source, 0, destination, 0, BACKLOG_BYTES);
        }
        self.renderer.queue.submit([encoder.finish()]);
        self.log.record("backlog-submitted", self.name, frame);
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct TargetError(&'static str);

pub struct Targets(Arc<Probe>);
pub struct Frame {
    probe: Arc<Probe>,
    view: wgpu::TextureView,
    size: [u32; 2],
    viewport: Option<Viewport>,
    number: u64,
}
impl TargetProvider for Targets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, renderer: &Renderer) -> std::result::Result<Frame, TargetError> {
        let probe = &self.0;
        let number = probe.acquires.fetch_add(1, Ordering::SeqCst) + 1;
        probe.log.enter_acquire();
        probe.log.record("acquire", probe.name, number);
        if probe.acquire_poll.swap(false, Ordering::SeqCst) {
            renderer
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .map_err(|_| TargetError("injected acquire poll failed"))?;
            probe.log.record("acquire-poll-end", probe.name, number);
        }
        if probe.fault.load(Ordering::SeqCst) == 1 {
            probe.log.record("acquire-refused", probe.name, number);
            return Err(TargetError("injected acquisition refusal"));
        }
        let texture = probe.texture.lock().expect("target");
        Ok(Frame {
            probe: Arc::clone(probe),
            view: texture.create_view(&Default::default()),
            size: [texture.width(), texture.height()],
            viewport: *probe.viewport.lock().expect("viewport"),
            number,
        })
    }
}
impl RenderFrame for Frame {
    type Error = TargetError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: self.size[0],
            height: self.size[1],
            viewport: self.viewport,
        }
    }
    fn present(self) -> std::result::Result<(), TargetError> {
        let probe = &self.probe;
        probe.log.record("present", probe.name, self.number);
        if probe.backlog_armed.swap(false, Ordering::SeqCst) {
            probe.submit_backlog(self.number);
        }
        // After any backlog: the consumer's callback that follows attaches to the
        // same latest submission and fires in the same batch, after this one.
        probe.witness(self.number);
        if probe.fault.load(Ordering::SeqCst) == 2 {
            probe.log.record("present-refused", probe.name, self.number);
            return Err(TargetError("injected presentation refusal"));
        }
        probe.presents.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
