//! Packet-local native MetalFX appearance review fixture for handoff 0074; not
//! production code. It is committed only under
//! handoffs/0074-metalfx-review/visual/review/ and copied into the granted child
//! checkout as crates/incant_render/tests/metalfx_visual_review.rs by
//! visual/tools/build.py, which first checks `SAMPLES` against that source.
//!
//! An extension of handoff 0072's reviewed temporal fixture (visual tree
//! ac8a48de): its stage, excluded-overlay scene, readback, edge and activity
//! helpers are reused byte for byte, which selfcheck.py verifies. Every case
//! opens one renderer with `Renderer::enable_metalfx` at startup and compares
//! same-source arms through `RetainedScene::submit`: off (no temporal state),
//! current-only (portable upscaling with history weight 0), portable (`REVIEW`)
//! and native (`RetainedScene::enable_metalfx`). Native frames must report
//! `Selection::MetalFx`; planned fractional or device-range fallbacks are
//! explicit expectations and never count as native. Pixels that the native
//! composite takes from the current frame (clear, fractional and mixed-depth
//! footprints) are reported as the engine's current-only boundary, not as
//! vendor output.
//!
//! Every world-frame submission (references and unkept frames included) and
//! every UI overlay submission is charged against hard per-case limits before
//! it is submitted, then read back. Only named frames and activity maps become
//! PNGs. Contract checks decide the outcome; image metrics are observations for
//! the reviewer and never pass or fail a case. No window, presentation, desktop
//! capture, audio or benchmark. Output goes only to the new directory named by
//! INCANT_0074_REVIEW_OUT.
#![cfg(feature = "native-metalfx")]
#![allow(dead_code)] // The shared fixture modules carry helpers these cases do not call.

mod materials;
mod support;

use glam::{DMat4, DQuat, DVec3, Vec3};
use incant_assets::{cook_gltf, cook_texture};
use incant_doc::{
    Asset, AssetImportSettings, Entity, MeshRenderer, SpriteMode, SpriteRenderer, SpriteSampling,
    TextureUsage, Transform, new_id,
};
use incant_render::{
    Renderer, RetainedScene, RetainedTarget, SceneInput, UiLayer, Viewport,
    metalfx::{Fallback, Preference, Selection},
    resolution::{ResolutionConfig, ResolutionScale},
    temporal::TemporalConfig,
    upscaling::UpscaleFrame,
    wgpu,
};
use incant_render_authoring::project_input;
use incant_render_data::{TextInput, TextNode, UiInput, UiRole, UiWidget};
use incant_text::{AtlasConfig, FontSource, GlyphAtlas, TextEngine, TextStyle, Wrap};
use incant_types::StableId;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SIZE: [u32; 2] = [960, 540];
const OUTPUT_ENV: &str = "INCANT_0074_REVIEW_OUT";
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
/// The text route's mixed-script control: a Latin ligature, an Arabic run,
/// CJK fallback and a Hebrew letter no packaged font covers.
const MIXED: &str = "ffi سلام 中文 ש";
/// Handoff 0072's explicit review policy, reused unchanged (selfcheck.py compares
/// the values); not a product preset or default. MetalFX's temporal weights are
/// opaque, so only the two depth tolerances reach the native composite's edge test.
const REVIEW: TemporalConfig = TemporalConfig {
    history_weight: 0.9,
    relative_depth: 0.05,
    absolute_depth: 0.02,
    reactive_threshold: 4.,
    reactive_strength: 0.5,
};
/// Current-only control: the portable path with no history contribution, so
/// its pixels are the jittered current frame resampled at output resolution.
const CURRENT_ONLY: TemporalConfig = TemporalConfig {
    history_weight: 0.,
    ..REVIEW
};
/// Copy of the engine's crate-private stratified jitter, in internal pixels,
/// positive right/down, applied as an image shift. build.py refuses a granted
/// source whose `temporal/jitter.rs` table differs.
const SAMPLES: [[f32; 2]; 8] = [
    [-0.375, -0.125],
    [0.125, 0.375],
    [0.375, -0.375],
    [-0.125, 0.125],
    [0.375, 0.125],
    [-0.125, -0.375],
    [-0.375, 0.375],
    [0.125, -0.125],
];
/// An integral 16:9 viewport inside the 960x540 target (still integral at 8/16).
const VIEWPORT: Viewport = Viewport {
    rect: [96., 54., 768., 432.],
    corner_radii: [0.; 4],
    canvas_srgb: [20, 21, 25],
};
/// The same placement off the pixel grid, which the native path must refuse.
const FRACTIONAL_VIEWPORT: Viewport = Viewport {
    rect: [96.5, 54.5, 767., 431.],
    ..VIEWPORT
};
/// A max-channel sRGB step above this between neighbors marks a reference edge.
const EDGE_THRESHOLD: u8 = 24;
/// A consecutive-frame change above this many sRGB steps counts as activity.
const CHANGE_THRESHOLD: u8 = 2;
const CAMERA_AT: [f64; 3] = [0.4, 4.2, 8.5];
const CAMERA_PITCH: f64 = -0.42;
const MOVER_START: [f64; 3] = [-2.6, 1., -1.];
const MOVER_STEP: f64 = 0.35;
const CARD_AT: [f64; 3] = [2.4, 1.15, -0.6];
const CARD_UV: [f32; 12] = [0., 1., 1., 1., 1., 0., 0., 1., 1., 0., 0., 0.];
const UI_RECT: [f32; 4] = [24., 24., 264., 80.];
/// UI stability window: the widget plus a 4 px margin, all over uniform sky.
const UI_PIXELS: [usize; 4] = [20, 20, 268, 84];

// ------------------------------------------------------------------ receipts

/// Append-only events plus one final report, in a directory that must be new.
/// Frame and UI-submission limits are hard: exceeding either panics before work.
struct Report {
    case: &'static str,
    root: PathBuf,
    events: File,
    frame_limit: u32,
    ui_limit: u32,
    frames: u32,
    ui: u32,
    records: Vec<Value>,
    pngs: Vec<Value>,
}
impl Report {
    fn open(case: &'static str, frame_limit: u32, ui_limit: u32) -> Self {
        let root = PathBuf::from(
            std::env::var_os(OUTPUT_ENV)
                .expect("INCANT_0074_REVIEW_OUT must name a new output directory"),
        );
        std::fs::create_dir(&root).expect("review output directory must not exist yet");
        let events = OpenOptions::new()
            .append(true)
            .create_new(true)
            .open(root.join("events.jsonl"))
            .unwrap();
        let mut report = Self {
            case,
            root,
            events,
            frame_limit,
            ui_limit,
            frames: 0,
            ui: 0,
            records: Vec::new(),
            pngs: Vec::new(),
        };
        report.event(json!({
            "event": "open",
            "case": case,
            "frame_limit": frame_limit,
            "ui_limit": ui_limit,
            "review_config": config_value(REVIEW),
        }));
        report
    }
    fn event(&mut self, value: Value) {
        writeln!(self.events, "{value}").unwrap();
        self.events.flush().unwrap();
    }
    /// Charged before submission, so a failed frame still counts.
    fn charge_frame(&mut self, name: &str) {
        self.frames += 1;
        assert!(
            self.frames <= self.frame_limit,
            "frame budget exceeded at {name}"
        );
        let index = self.frames;
        self.event(json!({"event": "frame", "name": name, "index": index}));
    }
    fn charge_ui(&mut self, name: &str) {
        self.ui += 1;
        assert!(
            self.ui <= self.ui_limit,
            "UI submission budget exceeded at {name}"
        );
        let index = self.ui;
        self.event(json!({"event": "ui", "name": name, "index": index}));
    }
    fn png(&mut self, name: &str, pixels: &[u8]) {
        assert_eq!(pixels.len(), (SIZE[0] * SIZE[1] * 4) as usize);
        let path = self.root.join(name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut encoder = png::Encoder::new(file, SIZE[0], SIZE[1]);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
        writer.finish().unwrap();
        let value = json!({
            "name": name,
            "sha256": hex(&std::fs::read(&path).unwrap()),
            "rgba_sha256": hex(pixels),
        });
        self.event(json!({"event": "png", "png": value}));
        self.pngs.push(value);
    }
    /// Writes report.json and returns whether every contract check passed.
    /// Observations are recorded but never decide the outcome.
    fn finish(mut self, mut body: Value, checks: &[(&str, bool)]) -> bool {
        let pass = checks.iter().all(|(_, ok)| *ok);
        let named: Map<String, Value> = checks
            .iter()
            .map(|(name, ok)| ((*name).to_owned(), json!(ok)))
            .collect();
        let object = body.as_object_mut().unwrap();
        object.insert("case".into(), json!(self.case));
        object.insert("checks".into(), Value::Object(named));
        object.insert("frames".into(), json!(self.records));
        object.insert("pngs".into(), json!(self.pngs));
        object.insert("frames_rendered".into(), json!(self.frames));
        object.insert("ui_submissions".into(), json!(self.ui));
        object.insert("review_config".into(), config_value(REVIEW));
        object.insert("outcome".into(), json!(if pass { "pass" } else { "fail" }));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join("report.json"))
            .unwrap();
        serde_json::to_writer_pretty(file, &body).unwrap();
        self.event(json!({"event": "finish", "pass": pass}));
        pass
    }
}
fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn config_value(config: TemporalConfig) -> Value {
    json!({
        "history_weight": config.history_weight,
        "relative_depth": config.relative_depth,
        "absolute_depth": config.absolute_depth,
        "reactive_threshold": config.reactive_threshold,
        "reactive_strength": config.reactive_strength,
    })
}

// ------------------------------------------------------------------ renderers

/// One device with MetalFX, portable upscaling and accumulation pipelines enabled
/// at startup; the backend must be the one named by INCANT_EXPECT_GPU_BACKEND
/// (tests/support `checked`). An unsupported device still opens here: its
/// admission refusal surfaces when a `RequireMetalFx` owner is enabled.
fn open_renderer() -> Renderer {
    let instance = wgpu::Instance::default();
    let created = pollster::block_on(Renderer::new(&instance, None));
    let mut renderer = support::checked(created.expect("renderer"));
    pollster::block_on(renderer.enable_metalfx()).expect("MetalFX and upscaling pipelines");
    renderer
}
fn timestamp_flags(features: wgpu::Features) -> Value {
    json!({
        "TIMESTAMP_QUERY": features.contains(wgpu::Features::TIMESTAMP_QUERY),
        "TIMESTAMP_QUERY_INSIDE_ENCODERS":
            features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
        "TIMESTAMP_QUERY_INSIDE_PASSES":
            features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES),
    })
}
fn describe(renderer: &Renderer) -> Value {
    let info = renderer.adapter.get_info();
    json!({
        "adapter": {
            "name": info.name,
            "vendor": info.vendor,
            "device": info.device,
            "device_type": format!("{:?}", info.device_type),
            "driver": info.driver,
            "driver_info": info.driver_info,
            "backend": format!("{:?}", info.backend),
        },
        "adapter_timestamp_features": timestamp_flags(renderer.adapter.features()),
        "device_timestamp_features": timestamp_flags(renderer.device.features()),
        "queue_timestamp_period_ns": renderer.queue.get_timestamp_period(),
        "timing_stats": stats_value(renderer),
    })
}
fn stats_value(renderer: &Renderer) -> Value {
    match renderer.gpu_timing_stats() {
        Ok(stats) => json!(stats),
        Err(error) => json!({"error": format!("{error:?}")}),
    }
}
fn plain(renderer: &Renderer, input: &SceneInput, camera: &str) -> RetainedScene {
    renderer
        .prepare_retained(input, camera)
        .expect("prepare retained scene")
}
/// The reviewed portable output-resolution path; it has no MetalFX owner.
fn portable(
    renderer: &Renderer,
    input: &SceneInput,
    camera: &str,
    config: TemporalConfig,
) -> RetainedScene {
    let mut scene = plain(renderer, input, camera);
    scene
        .enable_temporal_upscaling(renderer, config)
        .expect("enable portable upscaling");
    scene
}
/// A MetalFX owner over the same portable state. `RequireMetalFx` refuses an
/// unadmitted device here; `PreferMetalFx` reports each fallback per frame.
fn native(
    renderer: &Renderer,
    input: &SceneInput,
    camera: &str,
    preference: Preference,
) -> RetainedScene {
    let mut scene = plain(renderer, input, camera);
    scene
        .enable_metalfx(renderer, preference, REVIEW)
        .expect("enable MetalFX owner");
    scene
}
/// Fixed dynamic resolution, as in handoff 0072's transitions case.
fn fixed(sixteenths: u8) -> ResolutionConfig {
    let scale = ResolutionScale::new(sixteenths).expect("fixed scale");
    ResolutionConfig {
        target_ms: 16.,
        deadband: 0.1,
        minimum: scale,
        maximum: scale,
        initial: scale,
        overload_samples: 2,
        recovery_samples: 2,
        cooldown_samples: 1,
        slew: 1,
    }
}

// ------------------------------------------------------------------ frames

/// One 960x540 target; frames drawn into it use its fixed optional viewport.
struct Output {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    viewport: Option<Viewport>,
}
impl Output {
    fn new(renderer: &Renderer, viewport: Option<Viewport>) -> Self {
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("0074 MetalFX review target"),
            size: wgpu::Extent3d {
                width: SIZE[0],
                height: SIZE[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Self {
            texture,
            view,
            viewport,
        }
    }
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: SIZE[0],
            height: SIZE[1],
            viewport: self.viewport,
        }
    }
}
/// Accepted upscaling metadata of one submitted frame (portable and native).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Meta {
    phase: Option<u32>,
    sequence: u64,
    epoch: u64,
    previous: bool,
    internal: [u32; 2],
    bypassed: bool,
    output: [u32; 2],
}
impl Meta {
    fn of(frame: &UpscaleFrame) -> Self {
        let motion = frame.motion();
        Self {
            phase: motion.jitter_phase(),
            sequence: motion.sequence(),
            epoch: motion.epoch(),
            previous: motion.has_previous(),
            internal: motion.internal_size(),
            bypassed: motion.bypassed(),
            output: frame.output_size(),
        }
    }
    fn json(self) -> Value {
        json!({
            "jitter_phase": self.phase,
            "sequence": self.sequence,
            "epoch": self.epoch,
            "has_previous": self.previous,
            "internal_size": self.internal,
            "bypassed": self.bypassed,
            "output_size": self.output,
        })
    }
}
struct Shot {
    name: String,
    pixels: Vec<u8>,
    upscale: Option<UpscaleFrame>,
    /// The accepted MetalFX selection; None for scenes without a MetalFX owner.
    selection: Option<Selection>,
}
impl Shot {
    fn meta(&self) -> Option<Meta> {
        self.upscale.as_ref().map(Meta::of)
    }
}
/// One accepted retained frame, the optional UI overlay, then a blocking readback.
fn shoot(
    report: &mut Report,
    renderer: &Renderer,
    scene: &mut RetainedScene,
    input: &SceneInput,
    output: &Output,
    ui: Option<(&mut UiLayer, &Arc<UiInput>)>,
    name: &str,
) -> Shot {
    report.charge_frame(name);
    scene
        .submit(renderer, input, output.target())
        .expect("retained submit");
    if let Some((layer, ui)) = ui {
        report.charge_ui(name);
        let stage = layer.stage(renderer, ui).expect("UI stage");
        layer
            .submit(renderer, stage, &output.target())
            .expect("UI submit");
    }
    let pixels = read(renderer, &output.texture);
    let upscale = scene.upscale_frame();
    let selection = scene.metalfx_selection();
    let meta = upscale.as_ref().map(Meta::of);
    report.records.push(json!({
        "name": name,
        "upscale": meta.map(Meta::json),
        "metalfx_selection": selection.map(|s| format!("{s:?}")),
        "rgba_sha256": hex(&pixels),
    }));
    Shot {
        name: name.to_owned(),
        pixels,
        upscale,
        selection,
    }
}
/// `count` frames of one owner over an unchanged input, named `<prefix>-NN`.
fn run(
    report: &mut Report,
    renderer: &Renderer,
    mut scene: RetainedScene,
    input: &SceneInput,
    output: &Output,
    prefix: &str,
    count: u32,
) -> Vec<Shot> {
    (1..=count)
        .map(|k| {
            let name = format!("{prefix}-{k:02}");
            shoot(report, renderer, &mut scene, input, output, None, &name)
        })
        .collect()
}
fn read(renderer: &Renderer, texture: &wgpu::Texture) -> Vec<u8> {
    let [width, height] = SIZE;
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("0072 temporal review readback"),
        size: u64::from(stride) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .expect("wait for frame");
    receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("readback callback")
        .expect("readback mapping");
    let data = buffer.slice(..).get_mapped_range();
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in data.chunks(stride as usize) {
        pixels.extend_from_slice(&row[..(width * 4) as usize]);
    }
    drop(data);
    buffer.unmap();
    pixels
}
/// One uninterrupted accumulation run: jitter phases advance from zero, only
/// the first frame lacks history, the owner, epoch and internal extent stay
/// fixed and the accepted sequence advances by one per frame.
fn continuous(shots: &[Shot], internal: [u32; 2]) -> bool {
    let metas: Vec<Option<Meta>> = shots.iter().map(Shot::meta).collect();
    let Some(Some(first)) = metas.first().copied() else {
        return false;
    };
    metas.iter().copied().enumerate().all(|(i, meta)| {
        meta.is_some_and(|m| {
            m.phase == Some(i as u32 % 8)
                && m.previous == (i > 0)
                && m.epoch == first.epoch
                && m.sequence == first.sequence + i as u64
                && m.internal == internal
                && !m.bypassed
        })
    }) && same_owner(shots)
}
fn same_owner(shots: &[Shot]) -> bool {
    let Some(first) = shots.first().and_then(|s| s.upscale.as_ref()) else {
        return false;
    };
    shots.iter().all(|s| {
        s.upscale
            .as_ref()
            .is_some_and(|u| u.motion().same_owner(first.motion()))
    })
}
/// Every shot reports exactly this MetalFX selection (None: no MetalFX owner).
fn selected(shots: &[Shot], expected: Option<Selection>) -> bool {
    !shots.is_empty() && shots.iter().all(|s| s.selection == expected)
}
/// Planned selection of one native-owner step.
#[derive(Clone, Copy, Debug)]
enum Expect {
    /// Integral footprint at content scale 1, admitted on this device class.
    Native,
    /// Integral footprint below scale 1: native, or the device range refuses it.
    NativeOrScale,
    /// Off the pixel grid: the portable path must be selected for this reason.
    Fractional,
}
impl Expect {
    fn allows(self, selection: Option<Selection>) -> bool {
        match self {
            Self::Native => selection == Some(Selection::MetalFx),
            Self::NativeOrScale => matches!(
                selection,
                Some(Selection::MetalFx | Selection::Portable(Some(Fallback::Scale)))
            ),
            Self::Fractional => {
                selection == Some(Selection::Portable(Some(Fallback::FractionalViewport)))
            }
        }
    }
}
/// Each step's frames share one selection that its expectation allows.
fn steps_as_planned(steps: &[(Expect, &[Shot])]) -> bool {
    steps.iter().all(|(expect, shots)| {
        shots.first().is_some_and(|first| {
            expect.allows(first.selection) && shots.iter().all(|s| s.selection == first.selection)
        })
    })
}
fn selection_value(shots: &[Shot]) -> Value {
    shots
        .iter()
        .map(|s| {
            json!({
                "frame": s.name,
                "metalfx_selection": s.selection.map(|v| format!("{v:?}")),
                "upscale": s.meta().map(Meta::json),
            })
        })
        .collect()
}

// ------------------------------------------------------------------ pixels

fn max_delta(a: &[u8], b: &[u8], i: usize) -> u8 {
    (0..3)
        .map(|c| a[i * 4 + c].abs_diff(b[i * 4 + c]))
        .max()
        .unwrap_or(0)
}
/// Reference edges (max-channel neighbor step above EDGE_THRESHOLD), dilated by
/// one pixel to cover the half-pixel jitter envelope.
fn edges(reference: &[u8]) -> Vec<bool> {
    let [width, height] = SIZE.map(|v| v as usize);
    let step = |i: usize, j: usize| {
        (0..3)
            .map(|c| reference[i * 4 + c].abs_diff(reference[j * 4 + c]))
            .max()
            .unwrap_or(0)
    };
    let mut raw = vec![false; width * height];
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            if x + 1 < width && step(i, i + 1) > EDGE_THRESHOLD {
                raw[i] = true;
                raw[i + 1] = true;
            }
            if y + 1 < height && step(i, i + width) > EDGE_THRESHOLD {
                raw[i] = true;
                raw[i + width] = true;
            }
        }
    }
    let mut dilated = raw.clone();
    for y in 0..height {
        for x in 0..width {
            if raw[y * width + x] {
                for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        dilated[ny * width + nx] = true;
                    }
                }
            }
        }
    }
    dilated
}
#[derive(Clone, Copy, Default)]
struct Tally {
    pixels: usize,
    changed: usize,
    sum: u64,
    max: u8,
}
impl Tally {
    fn add(&mut self, delta: u8) {
        self.pixels += 1;
        if delta > CHANGE_THRESHOLD {
            self.changed += 1;
        }
        self.sum += u64::from(delta);
        self.max = self.max.max(delta);
    }
    fn json(self) -> Value {
        let mean = if self.pixels == 0 {
            0.
        } else {
            self.sum as f64 / self.pixels as f64
        };
        json!({
            "pixels": self.pixels,
            "changed_over_threshold": self.changed,
            "mean_delta": mean,
            "max_delta": self.max,
        })
    }
}
/// Max-channel sRGB differences between two frames, split by the edge mask.
fn split(a: &[u8], b: &[u8], edge: &[bool]) -> Value {
    let mut tallies = [Tally::default(); 2];
    for (i, &is_edge) in edge.iter().enumerate() {
        tallies[usize::from(is_edge)].add(max_delta(a, b, i));
    }
    json!({
        "identical": a == b,
        "interior": tallies[0].json(),
        "edge": tallies[1].json(),
    })
}
fn consecutive(shots: &[Shot], edge: &[bool]) -> Value {
    Value::Array(
        shots
            .windows(2)
            .map(|pair| {
                json!({
                    "from": pair[0].name,
                    "to": pair[1].name,
                    "delta": split(&pair[0].pixels, &pair[1].pixels, edge),
                })
            })
            .collect(),
    )
}
/// Grayscale map of the largest consecutive max-channel change per pixel (x4).
fn activity(shots: &[Shot]) -> Vec<u8> {
    let mut peak = vec![0u8; (SIZE[0] * SIZE[1]) as usize];
    for pair in shots.windows(2) {
        for (i, value) in peak.iter_mut().enumerate() {
            *value = (*value).max(max_delta(&pair[0].pixels, &pair[1].pixels, i));
        }
    }
    peak.into_iter()
        .flat_map(|p| {
            let v = p.saturating_mul(4);
            [v, v, v, 255]
        })
        .collect()
}
/// Largest max-channel difference inside a pixel rectangle [x0, y0, x1, y1).
fn rect_delta(a: &[u8], b: &[u8], rect: [usize; 4]) -> u8 {
    let width = SIZE[0] as usize;
    let mut worst = 0_u8;
    for y in rect[1]..rect[3] {
        for x in rect[0]..rect[2] {
            worst = worst.max(max_delta(a, b, y * width + x));
        }
    }
    worst
}

fn count(mask: &[bool]) -> usize {
    mask.iter().filter(|m| **m).count()
}
/// Pixels whose max-channel sRGB difference between two frames exceeds the
/// change threshold.
fn changed(a: &[u8], b: &[u8]) -> Vec<bool> {
    (0..a.len() / 4)
        .map(|i| max_delta(a, b, i) > CHANGE_THRESHOLD)
        .collect()
}
/// Max-channel differences between two frames inside each named mask (the first
/// mask containing a pixel wins) and in the remaining pixels.
fn regions(a: &[u8], b: &[u8], masks: &[(&str, &[bool])]) -> Value {
    let mut tallies = vec![Tally::default(); masks.len() + 1];
    for i in 0..a.len() / 4 {
        let region = masks
            .iter()
            .position(|(_, mask)| mask[i])
            .unwrap_or(masks.len());
        tallies[region].add(max_delta(a, b, i));
    }
    let mut named: Map<String, Value> = masks
        .iter()
        .zip(&tallies)
        .map(|((name, _), tally)| ((*name).to_owned(), tally.json()))
        .collect();
    named.insert("rest".into(), tallies[masks.len()].json());
    Value::Object(named)
}
/// Pixels whose centers lie more than one pixel outside `rect`, beyond the
/// display pass's antialiased chrome edge.
fn outside(rect: [f32; 4]) -> Vec<bool> {
    let [width, height] = SIZE.map(|v| v as usize);
    let columns = rect[0] - 1. ..=rect[0] + rect[2] + 1.;
    let rows = rect[1] - 1. ..=rect[1] + rect[3] + 1.;
    (0..width * height)
        .map(|i| {
            let x = (i % width) as f32 + 0.5;
            let y = (i / width) as f32 + 0.5;
            !(columns.contains(&x) && rows.contains(&y))
        })
        .collect()
}
/// Both frames hold identical RGBA bytes at every masked pixel.
fn identical_where(a: &[u8], b: &[u8], mask: &[bool]) -> bool {
    mask.iter()
        .enumerate()
        .all(|(i, &m)| !m || a[i * 4..i * 4 + 4] == b[i * 4..i * 4 + 4])
}

// ------------------------------------------------------------------ registration

/// Global sub-pixel translation of `frame` against `reference` over `mask`, in
/// output pixels, positive right/down: `frame` shows the reference content moved
/// by the result. One Lucas-Kanade step on mean sRGB codes with central
/// differences of the reference, meaningful only well under one pixel. None
/// when the masked gradients do not constrain both axes.
fn shift(reference: &[u8], frame: &[u8], mask: &[bool]) -> Option<[f64; 2]> {
    let [width, height] = SIZE.map(|v| v as usize);
    let luma = |pixels: &[u8], i: usize| {
        (f64::from(pixels[i * 4]) + f64::from(pixels[i * 4 + 1]) + f64::from(pixels[i * 4 + 2]))
            / 3.
    };
    let [mut gxx, mut gxy, mut gyy, mut gxe, mut gye] = [0_f64; 5];
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let i = y * width + x;
            if !mask[i] {
                continue;
            }
            let gx = (luma(reference, i + 1) - luma(reference, i - 1)) / 2.;
            let gy = (luma(reference, i + width) - luma(reference, i - width)) / 2.;
            let error = luma(frame, i) - luma(reference, i);
            gxx += gx * gx;
            gxy += gx * gy;
            gyy += gy * gy;
            gxe += gx * error;
            gye += gy * error;
        }
    }
    let det = gxx * gyy - gxy * gxy;
    if det.is_nan() || det <= 1e-9 * (gxx + gyy) * (gxx + gyy) {
        return None;
    }
    // frame ~ reference - shift . gradient, so (sum g g^T) shift = -(sum g error).
    Some([(gxy * gye - gyy * gxe) / det, (gxy * gxe - gxx * gye) / det])
}
/// Each shot's registration against the unjittered reference, paired with the
/// image shift its jitter phase applied to the current frame. A fitted slope of
/// estimated shift on image shift near zero means the jitter was undone; a
/// slope away from zero means frames keep (or reverse) their jitter. This is an
/// observation for the reviewer, judged together with the pixels.
fn registration(reference: &[u8], shots: &[Shot], mask: &[bool]) -> Value {
    let mut pairs: Vec<([f32; 2], [f64; 2])> = Vec::new();
    let frames: Vec<Value> = shots
        .iter()
        .map(|shot| {
            let phase = shot.meta().and_then(|m| m.phase);
            let image_shift = phase.map(|p| SAMPLES[p as usize % SAMPLES.len()]);
            let estimate = shift(reference, &shot.pixels, mask);
            if let (Some(j), Some(d)) = (image_shift, estimate) {
                pairs.push((j, d));
            }
            json!({
                "frame": shot.name,
                "jitter_phase": phase,
                "image_shift_px": image_shift,
                "estimated_shift_px": estimate,
            })
        })
        .collect();
    let axis = |a: usize| {
        let n = pairs.len() as f64;
        let mean_j = pairs.iter().map(|(j, _)| f64::from(j[a])).sum::<f64>() / n;
        let mean_d = pairs.iter().map(|(_, d)| d[a]).sum::<f64>() / n;
        let sjj: f64 = pairs
            .iter()
            .map(|(j, _)| (f64::from(j[a]) - mean_j).powi(2))
            .sum();
        let sjd: f64 = pairs
            .iter()
            .map(|(j, d)| (f64::from(j[a]) - mean_j) * (d[a] - mean_d))
            .sum();
        let slope = if sjj > 0. { Some(sjd / sjj) } else { None };
        json!({"mean_estimated_shift_px": mean_d, "slope_on_image_shift": slope})
    };
    let fit = if pairs.len() < 2 {
        Value::Null
    } else {
        json!({"pairs": pairs.len(), "x": axis(0), "y": axis(1)})
    };
    json!({
        "method": "one Lucas-Kanade step per frame on mean sRGB codes over the mask",
        "frames": frames,
        "fit": fit,
    })
}
/// The engine's conservative current-coverage boundary: pixels where the native
/// frame equals the same-phase current-only control in every compared frame.
/// Both arms take clear, fractional and mixed-depth footprints from the current
/// frame, so this band is engine policy, not MetalFX output (flat interiors can
/// also coincide). Reports its extent and the consecutive change of `later`
/// native frames inside it, on other edges and in other interiors.
fn current_band(native: &[Shot], current: &[Shot], edge: &[bool], later: &[Shot]) -> Value {
    let compared: Vec<(&Shot, &Shot)> = native
        .iter()
        .zip(current)
        .filter(|(n, c)| n.meta().map(|m| m.phase) == c.meta().map(|m| m.phase))
        .collect();
    let band: Vec<bool> = (0..edge.len())
        .map(|i| {
            !compared.is_empty()
                && compared
                    .iter()
                    .all(|(n, c)| n.pixels[i * 4..i * 4 + 3] == c.pixels[i * 4..i * 4 + 3])
        })
        .collect();
    let mut tallies = [Tally::default(); 3];
    for pair in later.windows(2) {
        for (i, (&in_band, &is_edge)) in band.iter().zip(edge).enumerate() {
            let region = if in_band {
                0
            } else if is_edge {
                1
            } else {
                2
            };
            tallies[region].add(max_delta(&pair[0].pixels, &pair[1].pixels, i));
        }
    }
    json!({
        "compared_frames": compared.iter().map(|(n, _)| n.name.clone()).collect::<Vec<_>>(),
        "band_pixels": count(&band),
        "band_edge_pixels": band.iter().zip(edge).filter(|(b, e)| **b && **e).count(),
        "later_consecutive": {
            "band": tallies[0].json(),
            "edge_outside_band": tallies[1].json(),
            "interior_outside_band": tallies[2].json(),
        },
    })
}
/// Frames whose native history must reset under the source's rules, inferred
/// only from public observables because no reset flag is exposed: the owner's
/// first native frame, a frame without valid motion history (cut), a content
/// size change, and the first native frame after a portable one.
fn inferred_resets(groups: &[&[Shot]]) -> Value {
    let mut previous: Option<&Shot> = None;
    let mut rows = Vec::new();
    for shot in groups.iter().flat_map(|group| group.iter()) {
        if shot.selection == Some(Selection::MetalFx) {
            let meta = shot.meta();
            let mut reasons = Vec::new();
            match previous {
                None => reasons.push("first native frame of the owner"),
                Some(last) => {
                    if last.selection != Some(Selection::MetalFx) {
                        reasons.push("previous frame was portable");
                    }
                    if last.meta().map(|m| m.internal) != meta.map(|m| m.internal) {
                        reasons.push("content size changed");
                    }
                }
            }
            if meta.is_some_and(|m| !m.previous) {
                reasons.push("no valid motion history");
            }
            if !reasons.is_empty() {
                rows.push(json!({"frame": shot.name, "reasons": reasons}));
            }
        }
        previous = Some(shot);
    }
    Value::Array(rows)
}

// ------------------------------------------------------------------ fixtures

/// Shadow look-dev geometry builders, adapted from handoff 0068's review helper
/// (originally tests/shadows/lookdev.rs, private to the model_gpu suite).
#[derive(Default)]
struct Mesh {
    positions: Vec<f32>,
    normals: Vec<f32>,
}
impl Mesh {
    fn triangle(&mut self, v: [Vec3; 3], n: [Vec3; 3]) {
        let face = (v[1] - v[0]).cross(v[2] - v[0]);
        let order = if face.dot(n[0] + n[1] + n[2]) >= 0. {
            [0, 1, 2]
        } else {
            [0, 2, 1]
        };
        for i in order {
            self.positions.extend(v[i].to_array());
            self.normals.extend(n[i].normalize().to_array());
        }
    }
    fn quad(&mut self, v: [Vec3; 4], n: Vec3) {
        self.triangle([v[0], v[1], v[2]], [n; 3]);
        self.triangle([v[0], v[2], v[3]], [n; 3]);
    }
    fn tilted_quad(&mut self, center: Vec3, u: Vec3, v: Vec3) {
        let n = u.cross(v).normalize();
        self.quad(
            [
                center - u - v,
                center + u - v,
                center + u + v,
                center - u + v,
            ],
            n,
        );
    }
    fn cuboid(&mut self, center: Vec3, half: Vec3) {
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            for sign in [-1., 1.] {
                let n = axis * sign;
                let (a, b) = (
                    n.any_orthonormal_vector(),
                    n.cross(n.any_orthonormal_vector()),
                );
                let (u, v) = (a * half.dot(a.abs()), b * half.dot(b.abs()));
                self.tilted_quad(center + n * half.dot(axis), u, v);
            }
        }
    }
    fn sphere(&mut self, center: Vec3, radius: f32, rings: u32, segments: u32) {
        let point = |i: u32, j: u32| {
            let theta = std::f32::consts::PI * i as f32 / rings as f32;
            let phi = std::f32::consts::TAU * j as f32 / segments as f32;
            Vec3::new(
                theta.sin() * phi.cos(),
                theta.cos(),
                theta.sin() * phi.sin(),
            )
        };
        for i in 0..rings {
            for j in 0..segments {
                let n = [
                    point(i, j),
                    point(i + 1, j),
                    point(i + 1, j + 1),
                    point(i, j + 1),
                ];
                let v = n.map(|n| center + n * radius);
                if i + 1 != rings {
                    self.triangle([v[0], v[1], v[2]], [n[0], n[1], n[2]]);
                }
                if i != 0 {
                    self.triangle([v[0], v[2], v[3]], [n[0], n[2], n[3]]);
                }
            }
        }
    }
}
/// Ground, cube, sphere, ramp with a thin post, a thin wall and 19 thin fence
/// posts receding to sub-pixel width: silhouettes and thin opaque detail.
fn lookdev_mesh() -> Mesh {
    let mut m = Mesh::default();
    m.tilted_quad(
        Vec3::new(0., 0., -50.),
        Vec3::new(14., 0., 0.),
        Vec3::new(0., 0., -60.),
    );
    m.cuboid(Vec3::new(-2.2, 0.5, 0.5), Vec3::splat(0.5));
    m.sphere(Vec3::new(0.6, 0.8, 1.0), 0.8, 24, 48);
    let slope = 25_f32.to_radians();
    let up_slope = Vec3::new(0., slope.sin(), -slope.cos());
    let ramp_center = Vec3::new(3.4, 1.6 * slope.sin(), -1.2 - 1.6 * slope.cos());
    m.tilted_quad(ramp_center, Vec3::new(1.3, 0., 0.), up_slope * 1.6);
    m.cuboid(
        ramp_center + Vec3::new(0., 0.6, 0.),
        Vec3::new(0.06, 0.75, 0.06),
    );
    m.cuboid(Vec3::new(-0.6, 0.6, -2.2), Vec3::new(1.4, 0.6, 0.01));
    for k in 0..19 {
        m.cuboid(
            Vec3::new(-4.5, 0.75, -4. - 4. * k as f32),
            Vec3::new(0.12, 0.75, 0.12),
        );
    }
    m
}
fn bounds(positions: &[f32]) -> ([f32; 3], [f32; 3]) {
    positions
        .chunks(3)
        .fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut lo, mut hi), p| {
            for i in 0..3 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
            (lo, hi)
        })
}
/// Writes `<name>.gltf` and `<name>.bin` (positions, normals, optional UVs and
/// a nearest-sampled image) with one material; nothing is cooked here.
fn write_model(
    root: &Path,
    name: &str,
    mesh: &Mesh,
    textured: Option<(&str, &[f32])>,
    material: Value,
) {
    let count = mesh.positions.len() / 3;
    let (min, max) = bounds(&mesh.positions);
    let mut bytes: Vec<u8> = mesh
        .positions
        .iter()
        .chain(&mesh.normals)
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let half = count * 12;
    let mut views = vec![
        json!({"buffer":0,"byteLength":half}),
        json!({"buffer":0,"byteOffset":half,"byteLength":half}),
    ];
    let mut accessors = vec![
        json!({"bufferView":0,"componentType":5126,"count":count,"type":"VEC3","min":min,"max":max}),
        json!({"bufferView":1,"componentType":5126,"count":count,"type":"VEC3"}),
    ];
    let mut attributes = json!({"POSITION":0,"NORMAL":1});
    let mut document = json!({"asset":{"version":"2.0"}});
    if let Some((image, uv)) = textured {
        assert_eq!(uv.len(), count * 2);
        let offset = bytes.len();
        bytes.extend(uv.iter().flat_map(|v| v.to_le_bytes()));
        views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":count * 8}));
        accessors.push(json!({"bufferView":2,"componentType":5126,"count":count,"type":"VEC2"}));
        attributes["TEXCOORD_0"] = json!(2);
        document["images"] = json!([{"uri": image}]);
        document["textures"] = json!([{"source":0,"sampler":0}]);
        document["samplers"] =
            json!([{"magFilter":9728,"minFilter":9728,"wrapS":33071,"wrapT":33071}]);
    }
    let buffer = format!("{name}.bin");
    std::fs::write(root.join(&buffer), &bytes).unwrap();
    document["buffers"] = json!([{"uri": buffer, "byteLength": bytes.len()}]);
    document["bufferViews"] = json!(views);
    document["accessors"] = json!(accessors);
    document["meshes"] = json!([{"primitives":[{"attributes": attributes, "material": 0}]}]);
    document["materials"] = json!([material]);
    document["nodes"] = json!([{"mesh":0}]);
    document["scenes"] = json!([{"nodes":[0]}]);
    document["scene"] = json!(0);
    std::fs::write(
        root.join(format!("{name}.gltf")),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
}
/// Cooks `<name>.gltf` beside the fixture's own model and registers the asset.
fn add_model(f: &mut materials::Fixture, name: &str) -> String {
    let cooked = cook_gltf(
        f.root.path(),
        Path::new(&format!("{name}.gltf")),
        &f.root.path().join(".incant/cache/models"),
    )
    .unwrap();
    let id = new_id();
    f.project.assets.insert(
        id.clone(),
        Asset {
            id: id.clone(),
            name: name.into(),
            path: format!("{name}.gltf"),
            kind: "model".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: None,
        },
    );
    id
}
fn insert(f: &mut materials::Fixture, entity: Entity) -> String {
    let id = entity.id.clone();
    f.project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .insert(id.clone(), entity);
    id
}
fn mesh_entity(name: &str, asset: &str, transform: Transform, cast_shadows: bool) -> Entity {
    let mut entity = Entity::new(name);
    entity
        .components
        .insert("Transform".into(), json!(transform));
    entity.components.insert(
        "MeshRenderer".into(),
        json!(MeshRenderer {
            mesh: asset.into(),
            materials: vec![],
            cast_shadows
        }),
    );
    entity
}
fn at(translation: [f64; 3], scale: f64) -> Transform {
    Transform {
        translation,
        scale: [scale; 3],
        ..Default::default()
    }
}
/// Moves the materials fixture's own quad entity.
fn place_fixture_quad(f: &mut materials::Fixture, transform: Transform) {
    let asset = f.asset_id.clone();
    let quad = f
        .project
        .scenes
        .values_mut()
        .next()
        .unwrap()
        .entities
        .values_mut()
        .find(|e| {
            e.components
                .get("MeshRenderer")
                .is_some_and(|m| m["mesh"] == asset.as_str())
        })
        .unwrap();
    quad.components.insert("Transform".into(), json!(transform));
}
fn camera_entity(name: &str, fov: f64, far: f64, translation: [f64; 3], rotation: DQuat) -> Entity {
    let mut camera = Entity::new(name);
    camera.components.insert(
        "Camera".into(),
        json!({"fov_degrees": fov, "near": 0.1, "far": far}),
    );
    camera.components.insert(
        "Transform".into(),
        json!(Transform {
            translation,
            rotation: rotation.to_array(),
            ..Default::default()
        }),
    );
    camera
}
fn sun_rotation(elevation_degrees: f64, azimuth_degrees: f64) -> [f64; 4] {
    (DQuat::from_rotation_y(azimuth_degrees.to_radians())
        * DQuat::from_rotation_x(-elevation_degrees.to_radians()))
    .to_array()
}
/// Environment fill and a shadowing sun; optionally two clustered point lights.
fn add_lights(f: &mut materials::Fixture, local: bool) {
    f.environment(&[34, 36, 44, 255].repeat(2), 2, 1, TextureUsage::Linear);
    let mut sun = Entity::new("Review sun");
    sun.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,0.96,0.9],"intensity":3.2,"shadows":{"distance":60}}),
    );
    sun.components.insert(
        "Transform".into(),
        json!(Transform {
            rotation: sun_rotation(38., 240.),
            ..Default::default()
        }),
    );
    insert(f, sun);
    if local {
        for (name, color, translation) in [
            ("Warm local light", [1.0, 0.62, 0.32], [-1.3, 1.1, 1.7]),
            ("Cool local light", [0.35, 0.6, 1.0], [2.2, 1.0, 0.4]),
        ] {
            let mut light = Entity::new(name);
            light.components.insert(
                "PointLight".into(),
                json!({"color":color,"intensity":6,"range":5}),
            );
            light
                .components
                .insert("Transform".into(), json!(at(translation, 1.)));
            insert(f, light);
        }
    }
}
/// 2x2 quad facing +Z with the materials fixture's UV layout.
fn card_mesh() -> Mesh {
    Mesh {
        positions: vec![
            -1., -1., 0., 1., -1., 0., 1., 1., 0., -1., -1., 0., 1., 1., 0., -1., 1., 0.,
        ],
        normals: [0_f32, 0., 1.].repeat(6),
    }
}
/// 128x128 MASK texture of one-texel bars and diagonals, mostly cut out; at the
/// review distance one texel is about one pixel.
fn grille() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128_u32 {
        for x in 0..128_u32 {
            let solid = x % 4 == 0 || y % 6 == 0 || (x + y) % 13 == 0;
            pixels.extend(if solid {
                [72, 186, 84, 255]
            } else {
                [0, 0, 0, 0]
            });
        }
    }
    pixels
}
/// 64x64 opaque checker of 8-texel squares, to reveal smears and lag.
fn checker() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(64 * 64 * 4);
    for y in 0..64_u32 {
        for x in 0..64_u32 {
            pixels.extend(if (x / 8 + y / 8) % 2 == 0 {
                [205, 205, 196, 255]
            } else {
                [58, 62, 74, 255]
            });
        }
    }
    pixels
}
/// The materials fixture quad as a lit, nearest-sampled checker backdrop.
fn checker_backdrop() -> materials::Fixture {
    let mut f = materials::Fixture::new(json!({
        "pbrMetallicRoughness": {"baseColorFactor":[1,1,1,1],"metallicFactor":0,"roughnessFactor":0.9}
    }));
    f.normal(Vec3::Z);
    f.texture(&checker(), 64, 64, |g| {
        g["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":0});
    });
    f
}
/// 16x16 badge with transparent corners and an asymmetric marker (shows flips).
fn badge() -> Vec<u8> {
    let mut pixels = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16_i32 {
        for x in 0..16_i32 {
            let d = (2 * x - 15).abs() + (2 * y - 15).abs();
            let texel = if d > 26 {
                [0, 0, 0, 0]
            } else if d > 22 {
                [40, 40, 48, 255]
            } else if x < 6 && y < 6 {
                [255, 210, 0, 255]
            } else if (x / 4 + y / 4) % 2 == 0 {
                [255, 255, 255, 255]
            } else {
                [200, 200, 210, 255]
            };
            pixels.extend(texel);
        }
    }
    pixels
}
/// Three nearest-sampled sprites in front of the backdrop (from handoff 0068).
fn add_sprites(f: &mut materials::Fixture) {
    materials::write_png(&f.root.path().join("badge.png"), &badge(), 16, 16);
    let cooked = cook_texture(
        f.root.path(),
        Path::new("badge.png"),
        &f.root.path().join(".incant/cache/textures"),
        TextureUsage::Color,
    )
    .unwrap();
    let texture = new_id();
    f.project.assets.insert(
        texture.clone(),
        Asset {
            id: texture.clone(),
            name: "Badge".into(),
            path: "badge.png".into(),
            kind: "texture".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: Some(AssetImportSettings::Texture {
                usage: TextureUsage::Color,
            }),
        },
    );
    for (index, (x, y, tint, flip_x, order)) in [
        (-1.0, -0.4, [1.0, 0.55, 0.45, 1.0], false, 0),
        (-0.15, -0.4, [0.55, 1.0, 0.6, 1.0], false, 0),
        (0.3, -0.3, [0.5, 0.7, 1.0, 1.0], true, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let mut sprite = Entity::new(format!("Sprite {index}"));
        sprite
            .components
            .insert("Transform".into(), json!(at([x, y, 0.3], 1.)));
        sprite.components.insert(
            "SpriteRenderer".into(),
            json!(SpriteRenderer {
                texture: texture.clone(),
                region: [0u32, 0, 16, 16].into(),
                size: [0.64, 0.64],
                pivot: [0.5, 0.5],
                tint,
                flip_x,
                flip_y: false,
                layer: 0,
                order,
                sampling: SpriteSampling::Nearest,
                mode: SpriteMode::Simple
            }),
        );
        insert(f, sprite);
    }
}
fn text_engine(fonts: &[&[u8]]) -> TextEngine {
    TextEngine::new(
        fonts
            .iter()
            .zip(1u128..)
            .map(|(bytes, id)| FontSource {
                id: id.into(),
                bytes: bytes.to_vec(),
            })
            .collect(),
    )
    .unwrap()
}
/// World-space mixed-script text (from handoff 0068's review helper).
fn text_input() -> TextInput {
    let mut engine = text_engine(&[LATIN, ARABIC, CJK]);
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 512,
        height: 256,
        max_glyphs: 512,
    })
    .unwrap();
    let nodes = [
        (1u8, MIXED, 20., [-1.5, 0.82]),
        (2, "Ag", 48., [0.35, 0.82]),
    ]
    .into_iter()
    .map(|(id, text, size, [x, y])| {
        let style = TextStyle {
            font_size: size,
            line_height: size * 1.4,
            width: 600.,
            wrap: Wrap::WordOrGlyph,
        };
        let layout = engine.shape(text, style).unwrap();
        let glyphs: Vec<_> = (0..layout.glyphs().len())
            .map(|i| engine.cache_glyph(&layout, i, &mut atlas).unwrap())
            .collect();
        TextNode {
            id: StableId([id; 16]),
            world: DMat4::from_translation(DVec3::new(x, y, 0.)),
            layout,
            glyphs: glyphs.into(),
        }
    })
    .collect();
    TextInput {
        nodes,
        atlas: Arc::new(atlas),
    }
}
/// One screen-space button with a label; drawn after the world frame and never
/// jittered, so its pixels must not change across jitter phases.
fn ui_input() -> Arc<UiInput> {
    let mut engine = text_engine(&[LATIN]);
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 256,
        height: 128,
        max_glyphs: 64,
    })
    .unwrap();
    let layout = engine
        .shape(
            "UI stays put",
            TextStyle {
                font_size: 18.,
                line_height: 24.,
                width: 220.,
                wrap: Wrap::WordOrGlyph,
            },
        )
        .unwrap();
    let glyphs: Vec<_> = (0..layout.glyphs().len())
        .map(|i| engine.cache_glyph(&layout, i, &mut atlas).unwrap())
        .collect();
    Arc::new(UiInput {
        viewport: [SIZE[0] as f32, SIZE[1] as f32],
        atlas: Arc::new(atlas),
        widgets: vec![UiWidget {
            id: StableId([7; 16]),
            role: UiRole::Button,
            disabled: false,
            focused: false,
            pressed: false,
            bounds: UI_RECT,
            visible: UI_RECT,
            origin: [UI_RECT[0] + 14., UI_RECT[1] + 14.],
            clip: [
                UI_RECT[0] + 8.,
                UI_RECT[1] + 6.,
                UI_RECT[2] - 8.,
                UI_RECT[3] - 6.,
            ],
            layout,
            glyphs: glyphs.into(),
            edit: None,
        }],
    })
}

// ------------------------------------------------------------------ scenes

/// Look-dev stage: thin opaque detail, a one-pixel MASK grille card, a bright
/// mover for object motion and disocclusion, a shadowing sun, two point lights
/// and environment fill. Shared by stationary, motion, transitions and timing.
struct Stage {
    f: materials::Fixture,
    camera: String,
    mover: String,
}
impl Stage {
    fn new() -> Self {
        let mut f = materials::Fixture::new(json!({}));
        write_model(
            f.root.path(),
            "quad",
            &lookdev_mesh(),
            None,
            json!({"pbrMetallicRoughness":{"baseColorFactor":[0.62,0.62,0.6,1],"metallicFactor":0,"roughnessFactor":0.85}}),
        );
        f.cook();
        add_lights(&mut f, true);
        let mut cube = Mesh::default();
        cube.cuboid(Vec3::ZERO, Vec3::splat(0.5));
        write_model(
            f.root.path(),
            "mover",
            &cube,
            None,
            json!({"pbrMetallicRoughness":{"baseColorFactor":[0.95,0.42,0.1,1],"metallicFactor":0,"roughnessFactor":0.45}}),
        );
        let mover_asset = add_model(&mut f, "mover");
        let mover = insert(
            &mut f,
            mesh_entity("Mover", &mover_asset, at(MOVER_START, 1.), true),
        );
        materials::write_png(&f.root.path().join("grille.png"), &grille(), 128, 128);
        write_model(
            f.root.path(),
            "card",
            &card_mesh(),
            Some(("grille.png", &CARD_UV[..])),
            json!({
                "pbrMetallicRoughness": {"baseColorFactor":[1,1,1,1],"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":0.8},
                "alphaMode": "MASK", "alphaCutoff": 0.5, "doubleSided": true
            }),
        );
        let card_asset = add_model(&mut f, "card");
        insert(
            &mut f,
            mesh_entity("Mask card", &card_asset, at(CARD_AT, 1.), true),
        );
        let camera = insert(
            &mut f,
            camera_entity(
                "Review camera",
                50.,
                120.,
                CAMERA_AT,
                DQuat::from_rotation_x(CAMERA_PITCH),
            ),
        );
        f.assets.sync_project(&f.project, f.root.path()).unwrap();
        Self { f, camera, mover }
    }
    fn input(&self) -> SceneInput {
        project_input(&self.f.project, Some(&self.f.assets)).unwrap()
    }
}
fn node(input: &SceneInput, id: &str) -> usize {
    input
        .nodes
        .iter()
        .position(|n| n.id == id)
        .expect("scene node")
}
fn mover_world(step: u32) -> DMat4 {
    DMat4::from_translation(DVec3::new(
        MOVER_START[0] + MOVER_STEP * f64::from(step),
        MOVER_START[1],
        MOVER_START[2],
    ))
}
/// Lateral camera translation with a slight yaw: parallax at every depth.
fn camera_world(step: u32) -> DMat4 {
    let k = f64::from(step);
    DMat4::from_rotation_translation(
        DQuat::from_rotation_y(-0.012 * k) * DQuat::from_rotation_x(CAMERA_PITCH),
        DVec3::new(CAMERA_AT[0] + 0.12 * k, CAMERA_AT[1], CAMERA_AT[2]),
    )
}
/// A discontinuous second viewpoint for the explicit cut.
fn view_b() -> DMat4 {
    DMat4::from_rotation_translation(
        DQuat::from_rotation_y(-0.55) * DQuat::from_rotation_x(-0.3),
        DVec3::new(-5., 3., 5.),
    )
}
/// Excluded surfaces over an accumulating opaque backdrop: a BLEND card (no
/// shadows), three sprites, world text and, at render time, a UI button over
/// uniform sky. Only the backdrop's opaque radiance enters history.
struct Excluded {
    f: materials::Fixture,
    camera: String,
}
impl Excluded {
    fn new() -> Self {
        let mut f = checker_backdrop();
        place_fixture_quad(&mut f, at([0., 0., -1.], 0.5));
        add_lights(&mut f, false);
        write_model(
            f.root.path(),
            "glass",
            &card_mesh(),
            None,
            materials::emissive([0.15, 0.45, 1.], 0.45, "BLEND"),
        );
        let glass = add_model(&mut f, "glass");
        insert(
            &mut f,
            mesh_entity("Blend glass", &glass, at([0.9, 0.25, 0.5], 0.45), false),
        );
        add_sprites(&mut f);
        let camera = insert(
            &mut f,
            camera_entity("Excluded camera", 40., 60., [0., 0., 6.], DQuat::IDENTITY),
        );
        f.assets.sync_project(&f.project, f.root.path()).unwrap();
        Self { f, camera }
    }
    fn input(&self) -> SceneInput {
        let mut input = project_input(&self.f.project, Some(&self.f.assets)).unwrap();
        input.text = Some(text_input());
        input
    }
}

// ------------------------------------------------------------------ cases

fn arms() -> Value {
    json!({
        "off": "RetainedScene without temporal state (handoff 0072 plain)",
        "current_only": {
            "route": "RetainedScene::enable_temporal_upscaling",
            "config": config_value(CURRENT_ONLY),
        },
        "portable": {
            "route": "RetainedScene::enable_temporal_upscaling",
            "config": config_value(REVIEW),
        },
        "native": {"route": "RetainedScene::enable_metalfx", "config": config_value(REVIEW)},
    })
}
/// One frame per name from one owner over an unchanged input.
fn named(
    report: &mut Report,
    renderer: &Renderer,
    scene: &mut RetainedScene,
    input: &SceneInput,
    output: &Output,
    names: &[&str],
) -> Vec<Shot> {
    names
        .iter()
        .map(|name| shoot(report, renderer, scene, input, output, None, name))
        .collect()
}
/// One owner over the mover's first eight steps, named `<prefix>-<step>`.
fn object_run(
    report: &mut Report,
    renderer: &Renderer,
    mut scene: RetainedScene,
    input: &mut SceneInput,
    mover: usize,
    output: &Output,
    prefix: &str,
) -> Vec<Shot> {
    (1..=8_u32)
        .map(|k| {
            input.nodes[mover].world = mover_world(k);
            let name = format!("{prefix}-{k}");
            shoot(report, renderer, &mut scene, input, output, None, &name)
        })
        .collect()
}
/// Every owner frame is from one motion owner.
fn one_owner(groups: &[&[Shot]]) -> bool {
    let Some(first) = groups
        .iter()
        .find_map(|group| group.first())
        .and_then(|s| s.upscale.as_ref())
    else {
        return false;
    };
    groups.iter().all(|group| {
        group.iter().all(|s| {
            s.upscale
                .as_ref()
                .is_some_and(|u| u.motion().same_owner(first.motion()))
        })
    })
}
fn internal_is(shots: &[Shot], size: [u32; 2]) -> bool {
    !shots.is_empty()
        && shots
            .iter()
            .all(|s| s.meta().is_some_and(|m| m.internal == size))
}

#[test]
#[ignore = "handoff 0074 MetalFX visual review; run only by the guarded packet runner"]
fn stationary() {
    let mut report = Report::open("stationary", 33, 0);
    let stage = Stage::new();
    let input = stage.input();
    let renderer = open_renderer();
    let output = Output::new(&renderer, None);
    let mut off = plain(&renderer, &input, &stage.camera);
    let scene_stats = json!(off.stats());
    let reference = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        None,
        "off",
    );
    drop(off);
    // Fresh owners in turn, so frame k of every arm uses jitter phase k-1.
    let scene = portable(&renderer, &input, &stage.camera, CURRENT_ONLY);
    let current = run(&mut report, &renderer, scene, &input, &output, "current", 8);
    let scene = portable(&renderer, &input, &stage.camera, REVIEW);
    let portable_shots = run(
        &mut report,
        &renderer,
        scene,
        &input,
        &output,
        "portable",
        8,
    );
    let scene = native(&renderer, &input, &stage.camera, Preference::RequireMetalFx);
    let native_shots = run(&mut report, &renderer, scene, &input, &output, "native", 16);
    let edge = edges(&reference.pixels);
    report.png("stationary-off.png", &reference.pixels);
    report.png("stationary-portable-f08.png", &portable_shots[7].pixels);
    report.png("stationary-native-f16.png", &native_shots[15].pixels);
    report.png(
        "stationary-native-activity.png",
        &activity(&native_shots[8..]),
    );
    let checks = [
        (
            "native_selected_every_frame",
            selected(&native_shots, Some(Selection::MetalFx)),
        ),
        (
            "controls_have_no_metalfx_owner",
            selected(&current, None)
                && selected(&portable_shots, None)
                && reference.selection.is_none(),
        ),
        (
            "native_run_uninterrupted_over_two_cycles",
            continuous(&native_shots, SIZE),
        ),
        (
            "portable_run_uninterrupted",
            continuous(&portable_shots, SIZE),
        ),
        ("current_only_run_uninterrupted", continuous(&current, SIZE)),
        (
            "reference_has_no_upscale_state",
            reference.upscale.is_none(),
        ),
        ("frames_match_plan", report.frames == 33),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "scene": scene_stats,
        "arms": arms(),
        "edge_mask": {
            "source": "off reference, 3x3 dilated",
            "threshold": EDGE_THRESHOLD,
            "pixels": count(&edge),
        },
        "selections": selection_value(&native_shots),
        "observations": {
            "registration_native_f09_f16": registration(&reference.pixels, &native_shots[8..], &edge),
            "registration_portable": registration(&reference.pixels, &portable_shots, &edge),
            "registration_current_only": registration(&reference.pixels, &current, &edge),
            "current_only_boundary": current_band(&native_shots[..8], &current, &edge, &native_shots[8..]),
            "native_consecutive": consecutive(&native_shots, &edge),
            "portable_consecutive": consecutive(&portable_shots, &edge),
            "native_same_phase_f08_vs_f16": split(&native_shots[7].pixels, &native_shots[15].pixels, &edge),
            "native_f16_vs_off": split(&reference.pixels, &native_shots[15].pixels, &edge),
            "portable_f08_vs_off": split(&reference.pixels, &portable_shots[7].pixels, &edge),
            "current_only_f08_vs_off": split(&reference.pixels, &current[7].pixels, &edge),
            "native_f08_vs_portable_f08": split(&portable_shots[7].pixels, &native_shots[7].pixels, &edge),
            "activity_map": "stationary-native-activity.png: native frames 9-16, max consecutive change x4",
        },
    });
    assert!(
        report.finish(body, &checks),
        "stationary failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0074 MetalFX visual review; run only by the guarded packet runner"]
fn motion() {
    let mut report = Report::open("motion", 34, 0);
    let stage = Stage::new();
    let mut input = stage.input();
    let mover = node(&input, &stage.mover);
    let camera = node(&input, &stage.camera);
    let renderer = open_renderer();
    let output = Output::new(&renderer, None);
    // The mover crosses in front of the thin wall and the MASK card, revealing
    // background; each arm is a fresh owner over the same eight steps.
    let scene = native(&renderer, &input, &stage.camera, Preference::RequireMetalFx);
    let native_object = object_run(
        &mut report,
        &renderer,
        scene,
        &mut input,
        mover,
        &output,
        "native",
    );
    let scene = portable(&renderer, &input, &stage.camera, REVIEW);
    let portable_object = object_run(
        &mut report,
        &renderer,
        scene,
        &mut input,
        mover,
        &output,
        "portable",
    );
    let scene = portable(&renderer, &input, &stage.camera, CURRENT_ONLY);
    let current_object = object_run(
        &mut report,
        &renderer,
        scene,
        &mut input,
        mover,
        &output,
        "current",
    );
    let mut off = plain(&renderer, &input, &stage.camera);
    let mut references = Vec::new();
    for k in 6..=8_u32 {
        input.nodes[mover].world = mover_world(k);
        let name = format!("off-{k}");
        references.push(shoot(
            &mut report,
            &renderer,
            &mut off,
            &input,
            &output,
            None,
            &name,
        ));
    }
    drop(off);
    // A fresh native owner pans the camera with the mover parked at step 8.
    let mut on = native(&renderer, &input, &stage.camera, Preference::RequireMetalFx);
    let mut pan = Vec::new();
    for k in 1..=6_u32 {
        input.nodes[camera].world = camera_world(k);
        let name = format!("pan-native-{k}");
        pan.push(shoot(
            &mut report,
            &renderer,
            &mut on,
            &input,
            &output,
            None,
            &name,
        ));
    }
    drop(on);
    let mut off = plain(&renderer, &input, &stage.camera);
    let pan_off = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        None,
        "pan-off-6",
    );
    drop(off);
    let [off6, off7, off8] = [&references[0], &references[1], &references[2]];
    // Pixels the mover covered, uncovered or shadowed between steps 7 and 8, and
    // those changed only since step 6: the disocclusion trail and its older part.
    let trail = changed(&off7.pixels, &off8.pixels);
    let older: Vec<bool> = changed(&off6.pixels, &off8.pixels)
        .iter()
        .zip(&trail)
        .map(|(o, t)| *o && !*t)
        .collect();
    let masks = [
        ("changed_7_to_8", trail.as_slice()),
        ("changed_6_to_8_only", older.as_slice()),
    ];
    let object_edge = edges(&off8.pixels);
    let pan_edge = edges(&pan_off.pixels);
    report.png("motion-native-f8.png", &native_object[7].pixels);
    report.png("motion-portable-f8.png", &portable_object[7].pixels);
    report.png("motion-off-f8.png", &off8.pixels);
    report.png("motion-pan-native-f6.png", &pan[5].pixels);
    report.png("motion-pan-off-f6.png", &pan_off.pixels);
    let checks = [
        (
            "native_selected_every_frame",
            selected(&native_object, Some(Selection::MetalFx))
                && selected(&pan, Some(Selection::MetalFx)),
        ),
        (
            "controls_have_no_metalfx_owner",
            selected(&portable_object, None)
                && selected(&current_object, None)
                && selected(&references, None)
                && pan_off.selection.is_none(),
        ),
        (
            "object_runs_uninterrupted",
            continuous(&native_object, SIZE)
                && continuous(&portable_object, SIZE)
                && continuous(&current_object, SIZE),
        ),
        ("pan_run_uninterrupted", continuous(&pan, SIZE)),
        (
            "references_have_no_upscale_state",
            references.iter().all(|s| s.upscale.is_none()) && pan_off.upscale.is_none(),
        ),
        ("frames_match_plan", report.frames == 34),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "arms": arms(),
        "mover_step_world_units": MOVER_STEP,
        "selections": {
            "native_object": selection_value(&native_object),
            "native_pan": selection_value(&pan),
        },
        "observations": {
            "trail_masks": {
                "changed_7_to_8": count(&trail),
                "changed_6_to_8_only": count(&older),
                "threshold": CHANGE_THRESHOLD,
            },
            "f8_vs_off_by_trail": {
                "native": regions(&off8.pixels, &native_object[7].pixels, &masks),
                "portable": regions(&off8.pixels, &portable_object[7].pixels, &masks),
                "current_only": regions(&off8.pixels, &current_object[7].pixels, &masks),
            },
            "current_only_boundary": current_band(&native_object, &current_object, &object_edge, &[]),
            "native_object_consecutive": consecutive(&native_object, &object_edge),
            "portable_object_consecutive": consecutive(&portable_object, &object_edge),
            "native_f8_vs_off": split(&off8.pixels, &native_object[7].pixels, &object_edge),
            "portable_f8_vs_off": split(&off8.pixels, &portable_object[7].pixels, &object_edge),
            "native_pan_consecutive": consecutive(&pan, &pan_edge),
            "native_pan_f6_vs_off": split(&pan_off.pixels, &pan[5].pixels, &pan_edge),
        },
    });
    assert!(
        report.finish(body, &checks),
        "motion failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0074 MetalFX visual review; run only by the guarded packet runner"]
fn excluded() {
    let mut report = Report::open("excluded", 11, 11);
    let scene = Excluded::new();
    let input = scene.input();
    let renderer = open_renderer();
    let output = Output::new(&renderer, None);
    let ui = ui_input();
    let mut layer = renderer.ui_layer();
    let mut on = native(&renderer, &input, &scene.camera, Preference::RequireMetalFx);
    // Prepared statistics, before any swap with the staging scene.
    let stats = on.stats().clone();
    let mut shots = Vec::new();
    for k in 1..=8_u32 {
        let name = format!("native-{k}");
        shots.push(shoot(
            &mut report,
            &renderer,
            &mut on,
            &input,
            &output,
            Some((&mut layer, &ui)),
            &name,
        ));
    }
    drop(on);
    let mut control = portable(&renderer, &input, &scene.camera, REVIEW);
    let mut portable_shots = Vec::new();
    for k in 1..=2_u32 {
        let name = format!("portable-{k}");
        portable_shots.push(shoot(
            &mut report,
            &renderer,
            &mut control,
            &input,
            &output,
            Some((&mut layer, &ui)),
            &name,
        ));
    }
    drop(control);
    let mut off = plain(&renderer, &input, &scene.camera);
    let reference = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        Some((&mut layer, &ui)),
        "off",
    );
    drop(off);
    let edge = edges(&reference.pixels);
    let ui_worst = shots
        .iter()
        .chain(&portable_shots)
        .map(|s| rect_delta(&s.pixels, &reference.pixels, UI_PIXELS))
        .max()
        .unwrap_or(u8::MAX);
    report.png("excluded-native-f8.png", &shots[7].pixels);
    report.png("excluded-off.png", &reference.pixels);
    report.png("excluded-native-activity.png", &activity(&shots[1..]));
    let checks = [
        (
            "native_selected_every_frame",
            selected(&shots, Some(Selection::MetalFx)),
        ),
        (
            "controls_have_no_metalfx_owner",
            selected(&portable_shots, None) && reference.selection.is_none(),
        ),
        ("native_run_uninterrupted", continuous(&shots, SIZE)),
        (
            "portable_run_uninterrupted",
            continuous(&portable_shots, SIZE),
        ),
        (
            "ui_stable_across_jitter_phases_arms_and_off",
            ui_worst <= CHANGE_THRESHOLD,
        ),
        (
            "scene_has_blend_sprites_and_text",
            stats.sprite_entities > 0 && stats.text_glyph_quads > 0 && stats.model_draw_calls >= 2,
        ),
        (
            "reference_has_no_upscale_state",
            reference.upscale.is_none(),
        ),
        ("frames_match_plan", report.frames == 11 && report.ui == 11),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "arms": arms(),
        "scene": stats,
        "ui": {"widget_rect": UI_RECT, "compared_pixels": UI_PIXELS, "max_delta_vs_off": ui_worst},
        "selections": selection_value(&shots),
        "observations": {
            "native_consecutive": consecutive(&shots, &edge),
            "native_f8_vs_off": split(&reference.pixels, &shots[7].pixels, &edge),
            "native_vs_portable_same_phase": [
                split(&portable_shots[0].pixels, &shots[0].pixels, &edge),
                split(&portable_shots[1].pixels, &shots[1].pixels, &edge),
            ],
            "ui_pixels_identical_to_off": shots
                .iter()
                .chain(&portable_shots)
                .map(|s| rect_delta(&s.pixels, &reference.pixels, UI_PIXELS) == 0)
                .collect::<Vec<_>>(),
            "activity_map": "excluded-native-activity.png: native frames 2-8, max consecutive change x4",
        },
    });
    assert!(
        report.finish(body, &checks),
        "excluded failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0074 MetalFX visual review; run only by the guarded packet runner"]
fn transitions() {
    let mut report = Report::open("transitions", 18, 0);
    let stage = Stage::new();
    let mut input = stage.input();
    let camera = node(&input, &stage.camera);
    let renderer = open_renderer();
    let output = Output::new(&renderer, None);
    // One preferring owner: view A, an explicit cut to view B, fixed 12/16 and
    // 8/16 content scales, the fractional 10/16 scale, then output scale again.
    let mut on = native(&renderer, &input, &stage.camera, Preference::PreferMetalFx);
    let view_a = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &output,
        &["a-1", "a-2", "a-3", "a-4"],
    );
    input.nodes[camera].world = view_b();
    on.invalidate_motion_history();
    let cut = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &output,
        &["cut-5", "b-6"],
    );
    on.set_dynamic_resolution(Some(fixed(12)))
        .expect("fixed 12/16 scale");
    let d12 = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &output,
        &["d12-7", "d12-8", "d12-9"],
    );
    on.set_dynamic_resolution(Some(fixed(8)))
        .expect("fixed 8/16 scale");
    let d8 = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &output,
        &["d8-10", "d8-11", "d8-12"],
    );
    on.set_dynamic_resolution(Some(fixed(10)))
        .expect("fixed 10/16 scale");
    let d10 = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &output,
        &["d10-13", "d10-14"],
    );
    on.set_dynamic_resolution(None)
        .expect("output-sized rendering");
    let resumed = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &output,
        &["full-15", "full-16"],
    );
    drop(on);
    let mut off = plain(&renderer, &input, &stage.camera);
    let reference = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        None,
        "off-b",
    );
    off.set_dynamic_resolution(Some(fixed(8)))
        .expect("fixed 8/16 reference scale");
    let reference_d8 = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        None,
        "off-b-d8",
    );
    drop(off);
    let edge = edges(&reference.pixels);
    report.png("transition-cut-f5.png", &cut[0].pixels);
    report.png("transition-d12-f7.png", &d12[0].pixels);
    report.png("transition-d8-f10.png", &d8[0].pixels);
    report.png("transition-resume-f15.png", &resumed[0].pixels);
    let groups = [
        view_a.as_slice(),
        cut.as_slice(),
        d12.as_slice(),
        d8.as_slice(),
        d10.as_slice(),
        resumed.as_slice(),
    ];
    let steps = [
        (Expect::Native, view_a.as_slice()),
        (Expect::Native, cut.as_slice()),
        (Expect::NativeOrScale, d12.as_slice()),
        (Expect::NativeOrScale, d8.as_slice()),
        (Expect::Fractional, d10.as_slice()),
        (Expect::Native, resumed.as_slice()),
    ];
    let epoch = |shots: &[Shot]| shots.first().and_then(Shot::meta).map(|m| m.epoch);
    let checks = [
        ("selections_as_planned", steps_as_planned(&steps)),
        (
            "one_owner_through_cut_scales_and_fallback",
            one_owner(&groups),
        ),
        ("view_a_run_uninterrupted", continuous(&view_a, SIZE)),
        ("cut_restarts_history_at_phase_zero", continuous(&cut, SIZE)),
        (
            "cut_advances_epoch",
            matches!((epoch(&view_a), epoch(&cut)), (Some(a), Some(b)) if b > a),
        ),
        (
            "internal_sizes_as_planned",
            internal_is(&view_a, SIZE)
                && internal_is(&cut, SIZE)
                && internal_is(&d12, [720, 405])
                && internal_is(&d8, [480, 270])
                && internal_is(&d10, [600, 338])
                && internal_is(&resumed, SIZE),
        ),
        (
            "references_have_no_upscale_state",
            reference.upscale.is_none() && reference_d8.upscale.is_none(),
        ),
        ("frames_match_plan", report.frames == 18),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "arms": arms(),
        "steps": [
            {"frames": "a-1..a-4", "sixteenths": 16, "expect": "Native"},
            {"frames": "cut-5, b-6", "sixteenths": 16, "expect": "Native", "cut": true},
            {"frames": "d12-7..d12-9", "sixteenths": 12, "expect": "NativeOrScale"},
            {"frames": "d8-10..d8-12", "sixteenths": 8, "expect": "NativeOrScale"},
            {"frames": "d10-13, d10-14", "sixteenths": 10, "expect": "Fractional"},
            {"frames": "full-15, full-16", "sixteenths": 16, "expect": "Native"},
        ],
        "selections": {
            "view_a": selection_value(&view_a),
            "cut": selection_value(&cut),
            "d12": selection_value(&d12),
            "d8": selection_value(&d8),
            "d10": selection_value(&d10),
            "resumed": selection_value(&resumed),
        },
        "observations": {
            "native_resets_inferred": inferred_resets(&groups),
            "cut_f5_vs_off_b": split(&reference.pixels, &cut[0].pixels, &edge),
            "b6_vs_off_b": split(&reference.pixels, &cut[1].pixels, &edge),
            "d12_first_vs_off_b": split(&reference.pixels, &d12[0].pixels, &edge),
            "d12_consecutive": consecutive(&d12, &edge),
            "d8_first_vs_off_b": split(&reference.pixels, &d8[0].pixels, &edge),
            "d8_last_vs_off_b": split(&reference.pixels, &d8[2].pixels, &edge),
            "d8_last_vs_unreconstructed_8_16": split(&reference_d8.pixels, &d8[2].pixels, &edge),
            "d8_consecutive": consecutive(&d8, &edge),
            "d10_last_vs_off_b": split(&reference.pixels, &d10[1].pixels, &edge),
            "resumed_f15_vs_off_b": split(&reference.pixels, &resumed[0].pixels, &edge),
            "resumed_f16_vs_off_b": split(&reference.pixels, &resumed[1].pixels, &edge),
        },
    });
    assert!(
        report.finish(body, &checks),
        "transitions failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0074 MetalFX visual review; run only by the guarded packet runner"]
fn viewport() {
    let mut report = Report::open("viewport", 12, 0);
    let stage = Stage::new();
    let input = stage.input();
    let renderer = open_renderer();
    let framed = Output::new(&renderer, Some(VIEWPORT));
    let fractional = Output::new(&renderer, Some(FRACTIONAL_VIEWPORT));
    // One preferring owner: the integral viewport at output scale, then at 8/16.
    let mut on = native(&renderer, &input, &stage.camera, Preference::PreferMetalFx);
    let full = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &framed,
        &[
            "framed-1", "framed-2", "framed-3", "framed-4", "framed-5", "framed-6",
        ],
    );
    on.set_dynamic_resolution(Some(fixed(8)))
        .expect("fixed 8/16 scale");
    let half = named(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &framed,
        &["framed-d8-7", "framed-d8-8", "framed-d8-9"],
    );
    drop(on);
    let mut off = plain(&renderer, &input, &stage.camera);
    let reference = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &framed,
        None,
        "off-framed",
    );
    drop(off);
    let mut off_grid = native(&renderer, &input, &stage.camera, Preference::PreferMetalFx);
    let fallback = named(
        &mut report,
        &renderer,
        &mut off_grid,
        &input,
        &fractional,
        &["fractional-1", "fractional-2"],
    );
    drop(off_grid);
    let outside_mask = outside(VIEWPORT.rect);
    let inside_edge: Vec<bool> = edges(&reference.pixels)
        .iter()
        .zip(&outside_mask)
        .map(|(e, o)| *e && !*o)
        .collect();
    let masks = [
        ("outside_viewport", outside_mask.as_slice()),
        ("inside_edges", inside_edge.as_slice()),
    ];
    report.png("viewport-native-f6.png", &full[5].pixels);
    report.png("viewport-off.png", &reference.pixels);
    let steps = [
        (Expect::Native, full.as_slice()),
        (Expect::NativeOrScale, half.as_slice()),
        (Expect::Fractional, fallback.as_slice()),
    ];
    let checks = [
        ("selections_as_planned", steps_as_planned(&steps)),
        ("framed_run_uninterrupted", continuous(&full, SIZE)),
        (
            "outside_viewport_matches_off",
            full.iter()
                .chain(&half)
                .all(|s| identical_where(&s.pixels, &reference.pixels, &outside_mask)),
        ),
        ("half_scale_internal_size", internal_is(&half, [480, 270])),
        (
            "reference_has_no_upscale_state",
            reference.upscale.is_none(),
        ),
        ("frames_match_plan", report.frames == 12),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "arms": arms(),
        "viewport": VIEWPORT.rect,
        "fractional_viewport": FRACTIONAL_VIEWPORT.rect,
        "canvas_srgb": VIEWPORT.canvas_srgb,
        "selections": {
            "framed": selection_value(&full),
            "framed_8_16": selection_value(&half),
            "fractional": selection_value(&fallback),
        },
        "observations": {
            "registration_native_inside_viewport": registration(&reference.pixels, &full, &inside_edge),
            "native_f6_vs_off": regions(&reference.pixels, &full[5].pixels, &masks),
            "native_8_16_last_vs_off": regions(&reference.pixels, &half[2].pixels, &masks),
            "native_consecutive_inside_edges": consecutive(&full, &inside_edge),
        },
    });
    assert!(
        report.finish(body, &checks),
        "viewport failed; see report.json"
    );
}
