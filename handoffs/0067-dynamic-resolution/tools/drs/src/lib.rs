//! Packet-local support for handoff 0067's dynamic-resolution GPU tests and
//! appearance helper. Public engine APIs only; nothing here is engine code, and
//! nothing here waits, queries GPU timestamps or changes renderer policy.
pub mod fixture;
pub mod pixels;
pub mod probe;

pub use incant_render::resolution::{
    ResolutionConfig, ResolutionError, ResolutionFrame, ResolutionScale,
};
pub use incant_render::{Renderer, RetainedTarget, Viewport, wgpu};
use serde_json::{Value, json};
use std::sync::Arc;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// The renderer's display-referred output backdrop (#141519, output.rs).
pub const BACKDROP_SRGB: [u8; 3] = [20, 21, 25];
/// The editor's viewport canvas colour (editor/app/src/main.rs).
pub const CANVAS_SRGB: [u8; 3] = [11, 12, 15];

pub fn scale(sixteenths: u8) -> ResolutionScale {
    ResolutionScale::new(sixteenths).expect("admitted scale")
}
/// Exact sixteenths of a receipt's scale (its fraction is n/16, exact in f32).
pub fn sixteenths(scale: ResolutionScale) -> u8 {
    (scale.fraction() * 16.).round() as u8
}
/// Fixed-scale control: identical policy fields, with minimum, initial and
/// maximum equal. The other fields are the renderer CPU tests' policy.
pub fn fixed(n: u8) -> ResolutionConfig {
    ResolutionConfig {
        target_ms: 16.,
        deadband: 0.125,
        minimum: scale(n),
        maximum: scale(n),
        initial: scale(n),
        overload_samples: 2,
        recovery_samples: 4,
        cooldown_samples: 2,
        slew: 1,
    }
}
/// Deterministic-direction lifecycle policy: every admitted sample below 500 ms
/// is under budget and raises the scale by one sixteenth (no cooldown), so each
/// accepted delivery is visible in the next receipt's scale.
pub fn stepping(initial: u8) -> ResolutionConfig {
    ResolutionConfig {
        target_ms: 1000.,
        deadband: 0.5,
        minimum: scale(4),
        maximum: ResolutionScale::FULL,
        initial: scale(initial),
        overload_samples: 240,
        recovery_samples: 1,
        cooldown_samples: 0,
        slew: 1,
    }
}
/// The separate bounded adaptive run: a 60 Hz-like target with streak,
/// cooldown and a two-sixteenth slew. Not a performance target or claim.
pub fn realistic() -> ResolutionConfig {
    ResolutionConfig {
        target_ms: 16.667,
        deadband: 0.1,
        minimum: scale(8),
        maximum: ResolutionScale::FULL,
        initial: scale(12),
        overload_samples: 3,
        recovery_samples: 2,
        cooldown_samples: 1,
        slew: 2,
    }
}
pub fn config_json(config: &ResolutionConfig) -> Value {
    json!({"target_ms": config.target_ms, "deadband": config.deadband,
        "minimum": sixteenths(config.minimum), "initial": sixteenths(config.initial),
        "maximum": sixteenths(config.maximum), "overload_samples": config.overload_samples,
        "recovery_samples": config.recovery_samples, "cooldown_samples": config.cooldown_samples,
        "slew": config.slew})
}
pub fn receipt_json(frame: Option<&ResolutionFrame>) -> Value {
    frame.map_or(Value::Null, |f| {
        json!({"sequence": f.sequence(), "scale16": sixteenths(f.scale()),
            "output": f.output_size(), "internal": f.internal_size(),
            "pixel_perfect": f.pixel_perfect()})
    })
}
/// ceil(output * n / 16) in exact integer arithmetic.
pub fn internal(output: [u32; 2], n: u8) -> [u32; 2] {
    output.map(|v| (v * u32::from(n)).div_ceil(16))
}

/// Offset, fractional camera viewport with the editor's rounded chrome.
pub fn offset_viewport() -> Viewport {
    Viewport {
        rect: [37.5, 21.25, 880.25, 495.5],
        corner_radii: [8.; 4],
        canvas_srgb: CANVAS_SRGB,
    }
}
/// The same rectangle scaled by n/16 for a disabled reference at the internal
/// size: square corners and a backdrop-coloured canvas, so chrome mixing over
/// uncovered texels is inert.
pub fn scaled_viewport(n: u8) -> Viewport {
    let factor = f32::from(n) / 16.;
    Viewport {
        rect: offset_viewport().rect.map(|v| v * factor),
        corner_radii: [0.; 4],
        canvas_srgb: BACKDROP_SRGB,
    }
}

/// A renderer on a test-owned instance, so wgpu's registry report is visible.
pub fn renderer() -> Result<(wgpu::Instance, Arc<Renderer>)> {
    let instance = wgpu::Instance::default();
    let renderer = pollster::block_on(Renderer::new(&instance, None))?;
    Ok((instance, Arc::new(renderer)))
}
pub fn adapter_json(renderer: &Renderer) -> Value {
    let info = renderer.adapter.get_info();
    json!({"name": info.name, "backend": format!("{:?}", info.backend),
        "device_type": format!("{:?}", info.device_type), "driver": info.driver,
        "driver_info": info.driver_info, "vendor": info.vendor, "device": info.device})
}
/// Live wgpu-core registry counts. Not a total driver-memory measurement.
pub fn registry_json(instance: &wgpu::Instance) -> Value {
    instance.generate_report().map_or(Value::Null, |r| {
        json!({"textures": r.hub.textures.num_allocated,
            "texture_views": r.hub.texture_views.num_allocated,
            "render_pipelines": r.hub.render_pipelines.num_allocated,
            "compute_pipelines": r.hub.compute_pipelines.num_allocated,
            "buffers": r.hub.buffers.num_allocated,
            "bind_groups": r.hub.bind_groups.num_allocated})
    })
}
/// One raw receipt line on stdout; the guarded runner retains the whole log.
pub fn emit(case: &str, kind: &str, body: Value) {
    println!(
        "DRS-RECEIPT {}",
        json!({"case": case, "kind": kind, "body": body})
    );
}

/// Frame-attempt ledger for one case: every consumer accept or retained submit
/// call counts, including refused ones that never reach the queue.
pub struct Budget {
    case: &'static str,
    declared: u32,
    attempts: u32,
    queue_submissions: u32,
}
impl Budget {
    pub fn new(case: &'static str, declared: u32) -> Self {
        Self {
            case,
            declared,
            attempts: 0,
            queue_submissions: 0,
        }
    }
    /// `submitted` records whether the attempt is known to reach the queue.
    pub fn attempt(&mut self, label: &str, submitted: bool) {
        self.attempts += 1;
        self.queue_submissions += u32::from(submitted);
        assert!(
            self.attempts <= self.declared,
            "{}: attempt {} ({label}) exceeds the frozen budget {}",
            self.case,
            self.attempts,
            self.declared
        );
    }
    pub fn finish(&self) {
        emit(
            self.case,
            "budget",
            json!({"declared_attempts": self.declared, "attempts": self.attempts,
                "frame_queue_submissions": self.queue_submissions}),
        );
        assert_eq!(
            self.attempts, self.declared,
            "{}: frame attempts differ from the frozen ledger",
            self.case
        );
    }
}

/// Collects contract failures so every receipt is printed before the verdict.
#[derive(Default)]
pub struct Verdict {
    failures: Vec<String>,
}
impl Verdict {
    pub fn check(&mut self, ok: bool, what: impl Into<String>) {
        if !ok {
            self.failures.push(what.into());
        }
    }
    pub fn finish(self, case: &str) {
        emit(case, "verdict", json!({"failures": self.failures}));
        assert!(self.failures.is_empty(), "{case}: {:?}", self.failures);
    }
}
