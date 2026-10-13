//! Packet-local temporal appearance review fixture for handoff 0072 stage B;
//! not production code. It is committed only under
//! handoffs/0072-motion-history-numeric/visual/review/ and copied into the
//! granted child checkout as crates/incant_render/tests/temporal_visual_review.rs
//! by visual/tools/build.py.
//!
//! Every case renders through `RetainedScene::submit` on a renderer that enabled
//! temporal accumulation at startup, with Claude's explicit `REVIEW` policy.
//! Temporal-off references are ordinary retained scenes on the same renderer.
//! Every world-frame submission (references and unkept frames included) and
//! every UI overlay submission is charged against hard per-case limits before
//! it is submitted, then read back. Only named frames and activity maps become
//! PNGs. Contract checks decide the outcome; image metrics are observations for
//! the reviewer and never pass or fail a case. No window, presentation, desktop
//! capture, audio or benchmark. Output goes only to the new directory named by
//! INCANT_0072_REVIEW_OUT.
#![allow(dead_code)] // The shared fixture modules carry helpers these cases do not call.

mod materials;
#[path = "../../incant_assets/tests/skeletal/fixture.rs"]
mod skeletal_fixture;
mod support;

use glam::{DMat4, DQuat, DVec3, Vec3};
use incant_assets::{cook_gltf, cook_texture};
use incant_assets_runtime::RuntimeAsset;
use incant_doc::{
    Asset, AssetImportSettings, Entity, MeshRenderer, SpriteMode, SpriteRenderer, SpriteSampling,
    TextureUsage, Transform, new_id,
};
use incant_render::{
    Renderer, RetainedScene, RetainedTarget, SceneInput, UiLayer,
    resolution::{ResolutionConfig, ResolutionScale},
    temporal::{
        MAX_TEMPORAL_COLOR_BYTES, MAX_TEMPORAL_TRANSITION_BYTES, TemporalConfig, TemporalFrame,
    },
    timing::{GpuFrameTiming, GpuPass, GpuSpan, GpuTimingError},
    wgpu,
};
use incant_render_authoring::project_input;
use incant_render_data::{MeshInput, SceneNode, TextInput, TextNode, UiInput, UiRole, UiWidget};
use incant_text::{AtlasConfig, FontSource, GlyphAtlas, TextEngine, TextStyle, Wrap};
use incant_types::{AnimationSample, CameraProjection, StableId};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SIZE: [u32; 2] = [960, 540];
const OUTPUT_ENV: &str = "INCANT_0072_REVIEW_OUT";
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
/// The text route's mixed-script control: a Latin ligature, an Arabic run,
/// CJK fallback and a Hebrew letter no packaged font covers.
const MIXED: &str = "ffi سلام 中文 ש";
/// Claude's explicit review policy for handoff 0072, validated before use; not a
/// product preset or default. Weight 0.9 converges within the 8-phase cycle while
/// bounding lag; 5% relative depth tolerance admits the per-pixel depth gradient
/// of grazing floors at 540 lines; reactivity engages only for large
/// scene-linear changes so it does not undo anti-aliasing of texture edges.
const REVIEW: TemporalConfig = TemporalConfig {
    history_weight: 0.9,
    relative_depth: 0.05,
    absolute_depth: 0.02,
    reactive_threshold: 4.,
    reactive_strength: 0.5,
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
const RIG_CAMERA: &str = "Rig camera";
const HALF_SCALE: u8 = 8;

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
                .expect("INCANT_0072_REVIEW_OUT must name a new output directory"),
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

/// One device with temporal pipelines enabled at startup; the backend must be
/// the one named by INCANT_EXPECT_GPU_BACKEND (tests/support `checked`).
fn open_renderer(timing: bool) -> Renderer {
    let instance = wgpu::Instance::default();
    let created = if timing {
        pollster::block_on(Renderer::new_with_gpu_timing(&instance, None))
    } else {
        pollster::block_on(Renderer::new(&instance, None))
    };
    let mut renderer = support::checked(created.expect("renderer"));
    pollster::block_on(renderer.enable_temporal_accumulation()).expect("temporal pipelines");
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
/// Drive callbacks without waiting until no measurement is pending (bounded).
fn settle(renderer: &Renderer) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut polls = 0;
    while renderer.gpu_timing_stats().is_ok_and(|s| s.pending > 0) && Instant::now() < deadline {
        renderer.check_device().expect("device");
        polls += 1;
        std::thread::sleep(Duration::from_millis(1));
    }
    polls
}
fn take(renderer: &Renderer) -> (Option<GpuFrameTiming>, Value) {
    match renderer.take_gpu_timing() {
        Ok(Some(sample)) => {
            let labeled: Map<String, Value> = GpuPass::ALL
                .iter()
                .zip(sample.passes)
                .map(|(pass, span)| (format!("{pass:?}"), json!(span)))
                .collect();
            let value = json!({"outcome": "sample", "raw": sample, "labeled": labeled});
            (Some(sample), value)
        }
        Ok(None) => (None, json!({"outcome": "none"})),
        Err(error) => {
            let typed = error
                .downcast_ref::<GpuTimingError>()
                .map(|e| format!("{e:?}"));
            (
                None,
                json!({"outcome": "error", "display": error.to_string(), "typed": typed}),
            )
        }
    }
}
fn plain(renderer: &Renderer, input: &SceneInput, camera: &str) -> RetainedScene {
    renderer
        .prepare_retained(input, camera)
        .expect("prepare retained scene")
}
fn temporal(renderer: &Renderer, input: &SceneInput, camera: &str) -> RetainedScene {
    let mut scene = plain(renderer, input, camera);
    scene
        .enable_temporal_history(renderer, REVIEW)
        .expect("enable temporal history");
    scene
}

// ------------------------------------------------------------------ frames

struct Output {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}
impl Output {
    fn new(renderer: &Renderer) -> Self {
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("0072 temporal review target"),
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
        Self { texture, view }
    }
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: FORMAT,
            width: SIZE[0],
            height: SIZE[1],
            viewport: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Meta {
    phase: Option<u32>,
    sequence: u64,
    epoch: u64,
    previous: bool,
    internal: [u32; 2],
    bypassed: bool,
}
impl Meta {
    fn of(frame: &TemporalFrame) -> Self {
        let motion = frame.motion();
        Self {
            phase: frame.jitter_phase(),
            sequence: motion.sequence(),
            epoch: motion.epoch(),
            previous: motion.has_previous(),
            internal: motion.internal_size(),
            bypassed: frame.bypassed(),
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
        })
    }
}
struct Shot {
    name: String,
    pixels: Vec<u8>,
    temporal: Option<TemporalFrame>,
}
impl Shot {
    fn meta(&self) -> Option<Meta> {
        self.temporal.as_ref().map(Meta::of)
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
    let temporal = scene.temporal_frame();
    let meta = temporal.as_ref().map(Meta::of);
    report.records.push(json!({
        "name": name,
        "temporal": meta.map(Meta::json),
        "rgba_sha256": hex(&pixels),
    }));
    Shot {
        name: name.to_owned(),
        pixels,
        temporal,
    }
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
    let Some(first) = shots.first().and_then(|s| s.temporal.as_ref()) else {
        return false;
    };
    shots.iter().all(|s| {
        s.temporal
            .as_ref()
            .is_some_and(|t| t.motion().same_owner(first.motion()))
    })
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

// ------------------------------------------------------------------ timing

fn group(sample: &GpuFrameTiming, pass: GpuPass) -> Option<GpuSpan> {
    GpuPass::ALL
        .iter()
        .position(|p| *p == pass)
        .and_then(|i| sample.passes[i])
}
fn median(mut values: Vec<f64>) -> Option<f64> {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(values[n / 2]),
        _ => Some((values[n / 2 - 1] + values[n / 2]) / 2.),
    }
}
/// Contract for one sample of the stage scene: its groups are present, sprite
/// and text groups are absent, spans are ordered and finite, period is valid.
fn sample_valid(sample: &GpuFrameTiming) -> bool {
    let present = |pass| group(sample, pass).is_some();
    let spans = sample
        .passes
        .iter()
        .flatten()
        .chain([&sample.frame])
        .all(|s| s.end_ticks >= s.start_ticks && s.nanoseconds.is_finite() && s.nanoseconds >= 0.);
    present(GpuPass::InitializeScene)
        && present(GpuPass::Shadows)
        && present(GpuPass::Models)
        && present(GpuPass::Display)
        && !present(GpuPass::Sprites)
        && !present(GpuPass::Text)
        && spans
        && sample.timestamp_period_ns.is_finite()
        && sample.timestamp_period_ns > 0.
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
            halos: None,
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
/// The skeletal fixture (exactly the asset the passing motion test uses) in
/// front of a checker backdrop, framed by an orthographic camera. Clip 1 rotates
/// the second joint, so two vertices deform while the first stays put.
struct Rig {
    f: materials::Fixture,
    rig: Arc<RuntimeAsset>,
}
impl Rig {
    fn new() -> Self {
        let mut f = checker_backdrop();
        place_fixture_quad(&mut f, at([11.6, 0.6, -0.6], 1.));
        add_lights(&mut f, false);
        f.assets.sync_project(&f.project, f.root.path()).unwrap();
        let root = f.root.path().join("rig");
        std::fs::create_dir(&root).unwrap();
        skeletal_fixture::fixture(&root);
        let cache = root.join("cache");
        let cooked = cook_gltf(&root, Path::new("model.gltf"), &cache).unwrap();
        let rig = Arc::new(
            RuntimeAsset::load_model("rig", 1, &cache, &cooked.metadata.fingerprint).unwrap(),
        );
        Self { f, rig }
    }
    fn input(&self, seconds: f64) -> SceneInput {
        let mut input = project_input(&self.f.project, Some(&self.f.assets)).unwrap();
        input.nodes.push(SceneNode {
            id: "Rig".into(),
            world: DMat4::IDENTITY,
            camera: None,
            light: None,
            mesh: Some(MeshInput {
                source: Arc::clone(&self.rig),
                cast_shadows: true,
                animation: Some(AnimationSample {
                    clip: Some(1),
                    seconds,
                }),
                animation_blend: None,
                animation_pose: None,
                animation_ik: None,
                animation_root: None,
            }),
            sprite: None,
            tilemap: None,
            diagnostic: false,
        });
        input.nodes.push(SceneNode {
            id: RIG_CAMERA.into(),
            world: DMat4::from_translation(DVec3::new(11.6, 0.6, 8.)),
            camera: Some(incant_types::Camera {
                fov_degrees: 50.,
                near: 0.1,
                far: 50.,
                projection: CameraProjection::Orthographic { vertical_size: 3. },
            }),
            light: None,
            mesh: None,
            sprite: None,
            tilemap: None,
            diagnostic: false,
        });
        input
    }
}

// ------------------------------------------------------------------ cases

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn probe() {
    let report = Report::open("probe", 0, 0);
    let instance = wgpu::Instance::default();
    let mut renderer =
        support::checked(pollster::block_on(Renderer::new(&instance, None)).expect("renderer"));
    let before = describe(&renderer);
    let enabled = pollster::block_on(renderer.enable_temporal_accumulation());
    let again = pollster::block_on(renderer.enable_temporal_accumulation());
    let invalid = [
        TemporalConfig {
            history_weight: 0.995,
            ..REVIEW
        },
        TemporalConfig {
            relative_depth: 0.2,
            ..REVIEW
        },
        TemporalConfig {
            absolute_depth: -0.01,
            ..REVIEW
        },
        TemporalConfig {
            reactive_threshold: 0.,
            ..REVIEW
        },
        TemporalConfig {
            reactive_strength: f32::NAN,
            ..REVIEW
        },
    ];
    let rejected: Vec<bool> = invalid.iter().map(|c| c.validate().is_err()).collect();
    drop(renderer);
    let timing = pollster::block_on(Renderer::new_with_gpu_timing(&instance, None));
    let timing_value = match &timing {
        Ok(timed) => describe(timed),
        Err(error) => json!({"refused": error.to_string()}),
    };
    let timing_available = timing.is_ok();
    drop(timing);
    let checks = [
        ("temporal_pipelines_enabled", enabled.is_ok()),
        ("second_enable_is_idempotent", again.is_ok()),
        ("review_config_validates", REVIEW.validate().is_ok()),
        ("invalid_configs_rejected", rejected.iter().all(|r| *r)),
    ];
    let body = json!({
        "renderer": before,
        "enable": format!("{enabled:?}"),
        "invalid_configs_rejected": rejected,
        "timing_renderer": timing_value,
        "observation_timing_renderer_available": timing_available,
        "limits": {
            "max_temporal_color_bytes": MAX_TEMPORAL_COLOR_BYTES,
            "max_temporal_transition_bytes": MAX_TEMPORAL_TRANSITION_BYTES,
        },
    });
    assert!(
        report.finish(body, &checks),
        "probe failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn stationary() {
    let mut report = Report::open("stationary", 17, 0);
    let stage = Stage::new();
    let input = stage.input();
    let renderer = open_renderer(false);
    let output = Output::new(&renderer);
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
    let mut on = temporal(&renderer, &input, &stage.camera);
    let mut shots = Vec::new();
    for k in 1..=16_u32 {
        let name = format!("on-{k:02}");
        shots.push(shoot(
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
    let edge = edges(&reference.pixels);
    report.png("stationary-off.png", &reference.pixels);
    report.png("stationary-on-f16.png", &shots[15].pixels);
    report.png("stationary-activity.png", &activity(&shots[8..]));
    let checks = [
        (
            "one_uninterrupted_run_over_two_cycles",
            continuous(&shots, SIZE),
        ),
        (
            "reference_has_no_temporal_state",
            reference.temporal.is_none(),
        ),
        ("frames_match_plan", report.frames == 17),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "scene": scene_stats,
        "edge_mask": {
            "source": "temporal-off reference, 3x3 dilated",
            "threshold": EDGE_THRESHOLD,
            "pixels": edge.iter().filter(|e| **e).count(),
        },
        "observations": {
            "consecutive": consecutive(&shots, &edge),
            "same_phase_f08_vs_f16": split(&shots[7].pixels, &shots[15].pixels, &edge),
            "f01_vs_off": split(&reference.pixels, &shots[0].pixels, &edge),
            "f16_vs_off": split(&reference.pixels, &shots[15].pixels, &edge),
            "activity_map": "stationary-activity.png: frames 9-16, max consecutive change x4",
        },
    });
    assert!(
        report.finish(body, &checks),
        "stationary failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn motion() {
    let mut report = Report::open("motion", 16, 0);
    let stage = Stage::new();
    let mut input = stage.input();
    let mover = node(&input, &stage.mover);
    let camera = node(&input, &stage.camera);
    let renderer = open_renderer(false);
    let output = Output::new(&renderer);
    // The mover crosses in front of the thin wall towards the sphere.
    let mut on = temporal(&renderer, &input, &stage.camera);
    let mut object = Vec::new();
    for k in 1..=8_u32 {
        input.nodes[mover].world = mover_world(k);
        let name = format!("object-on-{k}");
        object.push(shoot(
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
    let object_off = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        None,
        "object-off-8",
    );
    drop(off);
    // A fresh owner pans the camera with the mover parked at its last position.
    let mut on = temporal(&renderer, &input, &stage.camera);
    let mut pan = Vec::new();
    for k in 1..=6_u32 {
        input.nodes[camera].world = camera_world(k);
        let name = format!("pan-on-{k}");
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
    let object_edge = edges(&object_off.pixels);
    let pan_edge = edges(&pan_off.pixels);
    report.png("motion-object-on-f8.png", &object[7].pixels);
    report.png("motion-object-off-f8.png", &object_off.pixels);
    report.png("motion-pan-on-f6.png", &pan[5].pixels);
    report.png("motion-pan-off-f6.png", &pan_off.pixels);
    let checks = [
        ("object_run_uninterrupted", continuous(&object, SIZE)),
        ("pan_run_uninterrupted", continuous(&pan, SIZE)),
        (
            "references_have_no_temporal_state",
            object_off.temporal.is_none() && pan_off.temporal.is_none(),
        ),
        ("frames_match_plan", report.frames == 16),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "mover_step_world_units": MOVER_STEP,
        "observations": {
            "object_consecutive": consecutive(&object, &object_edge),
            "object_f8_vs_off": split(&object_off.pixels, &object[7].pixels, &object_edge),
            "pan_consecutive": consecutive(&pan, &pan_edge),
            "pan_f6_vs_off": split(&pan_off.pixels, &pan[5].pixels, &pan_edge),
        },
    });
    assert!(
        report.finish(body, &checks),
        "motion failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn excluded() {
    let mut report = Report::open("excluded", 9, 9);
    let scene = Excluded::new();
    let input = scene.input();
    let renderer = open_renderer(false);
    let output = Output::new(&renderer);
    let ui = ui_input();
    let mut layer = renderer.ui_layer();
    let mut on = temporal(&renderer, &input, &scene.camera);
    // Prepared statistics, before any swap with the staging scene.
    let stats = on.stats().clone();
    let mut shots = Vec::new();
    for k in 1..=8_u32 {
        let name = format!("on-{k}");
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
        .map(|s| rect_delta(&s.pixels, &reference.pixels, UI_PIXELS))
        .max()
        .unwrap_or(u8::MAX);
    report.png("excluded-on-f8.png", &shots[7].pixels);
    report.png("excluded-off.png", &reference.pixels);
    report.png("excluded-activity.png", &activity(&shots[1..]));
    let checks = [
        ("accumulation_run_uninterrupted", continuous(&shots, SIZE)),
        (
            "ui_stable_across_jitter_phases_and_off",
            ui_worst <= CHANGE_THRESHOLD,
        ),
        (
            "scene_has_blend_sprites_and_text",
            stats.sprite_entities > 0 && stats.text_glyph_quads > 0 && stats.model_draw_calls >= 2,
        ),
        (
            "reference_has_no_temporal_state",
            reference.temporal.is_none(),
        ),
        ("frames_match_plan", report.frames == 9 && report.ui == 9),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "scene": stats,
        "ui": {"widget_rect": UI_RECT, "compared_pixels": UI_PIXELS, "max_delta_vs_off": ui_worst},
        "observations": {
            "consecutive": consecutive(&shots, &edge),
            "f8_vs_off": split(&reference.pixels, &shots[7].pixels, &edge),
            "ui_pixels_identical_to_off": shots
                .iter()
                .map(|s| rect_delta(&s.pixels, &reference.pixels, UI_PIXELS) == 0)
                .collect::<Vec<_>>(),
            "activity_map": "excluded-activity.png: frames 2-8, max consecutive change x4",
        },
    });
    assert!(
        report.finish(body, &checks),
        "excluded failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn transitions() {
    let mut report = Report::open("transitions", 13, 0);
    let stage = Stage::new();
    let mut input = stage.input();
    let camera = node(&input, &stage.camera);
    let renderer = open_renderer(false);
    let output = Output::new(&renderer);
    let mut on = temporal(&renderer, &input, &stage.camera);
    let mut view_a = Vec::new();
    for k in 1..=6_u32 {
        let name = format!("a-{k}");
        view_a.push(shoot(
            &mut report,
            &renderer,
            &mut on,
            &input,
            &output,
            None,
            &name,
        ));
    }
    // Discontinuous cut to view B, signalled explicitly by the host.
    input.nodes[camera].world = view_b();
    on.invalidate_motion_history();
    let mut cut = Vec::new();
    for name in ["cut-7", "b-8"] {
        cut.push(shoot(
            &mut report,
            &renderer,
            &mut on,
            &input,
            &output,
            None,
            name,
        ));
    }
    // Fixed half-scale dynamic resolution, then output-sized rendering again.
    let half = ResolutionScale::new(HALF_SCALE).expect("half scale");
    on.set_dynamic_resolution(Some(ResolutionConfig {
        target_ms: 16.,
        deadband: 0.1,
        minimum: half,
        maximum: half,
        initial: half,
        overload_samples: 2,
        recovery_samples: 2,
        cooldown_samples: 1,
        slew: 1,
    }))
    .expect("fixed dynamic resolution");
    let mut scaled = Vec::new();
    for name in ["drs-9", "drs-10", "drs-11"] {
        scaled.push(shoot(
            &mut report,
            &renderer,
            &mut on,
            &input,
            &output,
            None,
            name,
        ));
    }
    on.set_dynamic_resolution(None)
        .expect("output-sized rendering");
    let restored = vec![shoot(
        &mut report,
        &renderer,
        &mut on,
        &input,
        &output,
        None,
        "full-12",
    )];
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
    drop(off);
    let edge = edges(&reference.pixels);
    report.png("transition-cut-f7.png", &cut[0].pixels);
    report.png("transition-drs-f9.png", &scaled[0].pixels);
    let epoch = |shots: &[Shot]| shots.first().and_then(Shot::meta).map(|m| m.epoch);
    let half_size = SIZE.map(|v| v * u32::from(HALF_SCALE) / 16);
    let owner = view_a
        .first()
        .and_then(|s| s.temporal.as_ref())
        .is_some_and(|first| {
            view_a
                .iter()
                .chain(&cut)
                .chain(&scaled)
                .chain(&restored)
                .all(|s| {
                    s.temporal
                        .as_ref()
                        .is_some_and(|t| t.motion().same_owner(first.motion()))
                })
        });
    let checks = [
        ("view_a_run_uninterrupted", continuous(&view_a, SIZE)),
        ("cut_restarts_history_at_phase_zero", continuous(&cut, SIZE)),
        (
            "cut_advances_epoch",
            matches!((epoch(&view_a), epoch(&cut)), (Some(a), Some(b)) if b > a),
        ),
        (
            "half_scale_restarts_history",
            continuous(&scaled, half_size),
        ),
        ("output_scale_restarts_history", continuous(&restored, SIZE)),
        ("one_owner_through_cut_and_scale_changes", owner),
        (
            "reference_has_no_temporal_state",
            reference.temporal.is_none(),
        ),
        ("frames_match_plan", report.frames == 13),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "half_scale_internal_size": half_size,
        "observations": {
            "epochs": {
                "view_a": epoch(&view_a),
                "cut": epoch(&cut),
                "drs": epoch(&scaled),
                "restored": epoch(&restored),
            },
            "cut_f7_vs_off_b": split(&reference.pixels, &cut[0].pixels, &edge),
            "b8_vs_off_b": split(&reference.pixels, &cut[1].pixels, &edge),
            "drs_consecutive": consecutive(&scaled, &edge),
            "restored_f12_vs_off_b": split(&reference.pixels, &restored[0].pixels, &edge),
        },
    });
    assert!(
        report.finish(body, &checks),
        "transitions failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn skinned() {
    let mut report = Report::open("skinned", 7, 0);
    let rig = Rig::new();
    let renderer = open_renderer(false);
    let output = Output::new(&renderer);
    let mut on = temporal(&renderer, &rig.input(0.), RIG_CAMERA);
    let mut shots = Vec::new();
    for k in 0..6_u32 {
        let input = rig.input(0.1 * f64::from(k));
        let name = format!("on-{}", k + 1);
        shots.push(shoot(
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
    let input = rig.input(0.5);
    let mut off = plain(&renderer, &input, RIG_CAMERA);
    let reference = shoot(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        None,
        "off-6",
    );
    drop(off);
    let edge = edges(&reference.pixels);
    report.png("skinned-on-f6.png", &shots[5].pixels);
    report.png("skinned-off-f6.png", &reference.pixels);
    let checks = [
        ("skinned_run_uninterrupted", continuous(&shots, SIZE)),
        (
            "reference_has_no_temporal_state",
            reference.temporal.is_none(),
        ),
        ("frames_match_plan", report.frames == 7),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "animation": {"clip": 1, "seconds": [0.0, 0.1, 0.2, 0.3, 0.4, 0.5]},
        "observations": {
            "consecutive": consecutive(&shots, &edge),
            "f6_vs_off": split(&reference.pixels, &shots[5].pixels, &edge),
            "f1_vs_f6_identical": shots[0].pixels == shots[5].pixels,
        },
    });
    assert!(
        report.finish(body, &checks),
        "skinned failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn timing() {
    let mut report = Report::open("timing", 8, 0);
    let stage = Stage::new();
    let input = stage.input();
    let renderer = open_renderer(true);
    let output = Output::new(&renderer);
    let mut off = plain(&renderer, &input, &stage.camera);
    let mut on = temporal(&renderer, &input, &stage.camera);
    let mut samples = Vec::new();
    let mut on_shots = Vec::new();
    // Interleaved so drift or warm-up affects both sides alike.
    for k in 1..=4_u32 {
        for accumulate in [false, true] {
            let name = format!("{}-{k}", if accumulate { "on" } else { "off" });
            let shot = if accumulate {
                shoot(
                    &mut report,
                    &renderer,
                    &mut on,
                    &input,
                    &output,
                    None,
                    &name,
                )
            } else {
                shoot(
                    &mut report,
                    &renderer,
                    &mut off,
                    &input,
                    &output,
                    None,
                    &name,
                )
            };
            let polls = settle(&renderer);
            let (sample, value) = take(&renderer);
            samples.push((
                accumulate,
                sample,
                json!({"frame": name, "settle_polls": polls, "take": value}),
            ));
            if accumulate {
                on_shots.push(shot);
            }
        }
    }
    let final_stats = stats_value(&renderer);
    drop(on);
    drop(off);
    let side = |accumulate: bool, pass: GpuPass| {
        median(
            samples
                .iter()
                .filter(|(a, _, _)| *a == accumulate)
                .filter_map(|(_, s, _)| s.as_ref().and_then(|s| group(s, pass)))
                .map(|span| span.nanoseconds)
                .collect(),
        )
    };
    let medians: Map<String, Value> = GpuPass::ALL
        .iter()
        .map(|&pass| {
            (
                format!("{pass:?}"),
                json!({"off": side(false, pass), "on": side(true, pass)}),
            )
        })
        .collect();
    let models = (side(true, GpuPass::Models), side(false, GpuPass::Models));
    let inside = samples.iter().all(|(_, s, _)| {
        s.as_ref().is_none_or(|s| {
            s.passes.iter().flatten().all(|span| {
                span.start_ticks >= s.frame.start_ticks && span.end_ticks <= s.frame.end_ticks
            })
        })
    });
    let checks = [
        (
            "every_frame_produced_one_sample",
            samples.iter().all(|(_, s, _)| s.is_some()),
        ),
        (
            "samples_well_formed_with_models_group",
            samples
                .iter()
                .all(|(_, s, _)| s.as_ref().is_some_and(sample_valid)),
        ),
        ("temporal_run_uninterrupted", continuous(&on_shots, SIZE)),
        (
            "models_span_grows_with_motion_and_resolve_passes",
            matches!(models, (Some(on), Some(off)) if on > off),
        ),
        ("frames_match_plan", report.frames == 8),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "samples": samples.iter().map(|(a, _, v)| json!({"temporal": a, "value": v})).collect::<Vec<_>>(),
        "median_nanoseconds": medians,
        "final_stats": final_stats,
        "observations": {
            "models_median_ratio_on_over_off": match models {
                (Some(on), Some(off)) if off > 0. => Some(on / off),
                _ => None,
            },
            "groups_within_frame": inside,
            "note": "Medians of four interleaved samples per side; inference about pass coverage, not a performance measurement.",
        },
    });
    assert!(
        report.finish(body, &checks),
        "timing failed; see report.json"
    );
}

// ------------------------------------------------------------------ fixed-sharpness comparison

/// Integer-offset, odd-sized viewport inside the unchanged 960x540 output. At full
/// scale FramePlan keeps this placement, so it is also the temporal rect.
const VIEWPORT_RECT: [f32; 4] = [37., 19., 885., 501.];
/// The neutral #141519 surround outside the viewport.
const VIEWPORT_CANVAS: [u8; 3] = [0x14, 0x15, 0x19];

fn viewport_target(output: &Output) -> RetainedTarget<'_> {
    RetainedTarget {
        viewport: Some(incant_render::Viewport {
            rect: VIEWPORT_RECT,
            corner_radii: [0.; 4],
            canvas_srgb: VIEWPORT_CANVAS,
        }),
        ..output.target()
    }
}
/// `shoot` into the offset viewport, without a UI overlay.
fn shoot_viewport(
    report: &mut Report,
    renderer: &Renderer,
    scene: &mut RetainedScene,
    input: &SceneInput,
    output: &Output,
    name: &str,
) -> Shot {
    report.charge_frame(name);
    scene
        .submit(renderer, input, viewport_target(output))
        .expect("retained submit in viewport");
    let pixels = read(renderer, &output.texture);
    let temporal = scene.temporal_frame();
    let meta = temporal.as_ref().map(Meta::of);
    report.records.push(json!({
        "name": name,
        "temporal": meta.map(Meta::json),
        "rgba_sha256": hex(&pixels),
    }));
    Shot {
        name: name.to_owned(),
        pixels,
        temporal,
    }
}
fn viewport_bounds() -> [usize; 4] {
    VIEWPORT_RECT.map(|v| v as usize)
}
/// Largest max-channel difference outside the viewport.
fn outside_delta(a: &[u8], b: &[u8]) -> u8 {
    let [width, height] = SIZE.map(|v| v as usize);
    let [x0, y0, w, h] = viewport_bounds();
    let mut worst = 0_u8;
    for y in 0..height {
        for x in 0..width {
            if (x0..x0 + w).contains(&x) && (y0..y0 + h).contains(&y) {
                continue;
            }
            worst = worst.max(max_delta(a, b, y * width + x));
        }
    }
    worst
}
/// Max-channel differences at non-edge viewport pixels, split into the 3 px band
/// just inside the viewport border and the rest of the viewport.
fn border_split(a: &[u8], b: &[u8], edge: &[bool]) -> Value {
    let width = SIZE[0] as usize;
    let [x0, y0, w, h] = viewport_bounds();
    let mut tallies = [Tally::default(); 2];
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let i = y * width + x;
            if edge[i] {
                continue;
            }
            let inner = (x0 + 3..x0 + w - 3).contains(&x) && (y0 + 3..y0 + h - 3).contains(&y);
            tallies[usize::from(!inner)].add(max_delta(a, b, i));
        }
    }
    json!({"inner": tallies[0].json(), "border_band_3px": tallies[1].json()})
}
/// Pixels of an activity map (peak consecutive change x4, saturating) per peak band.
fn activity_bins(map: &[u8]) -> Value {
    let mut bins = [0_usize; 6];
    for px in map.chunks(4) {
        let peak = if px[0] == 255 { 64 } else { px[0] / 4 };
        let band = match peak {
            0 => continue,
            1..=2 => 0,
            3..=8 => 1,
            9..=16 => 2,
            17..=32 => 3,
            33..=63 => 4,
            _ => 5,
        };
        bins[band] += 1;
    }
    json!({
        "1-2": bins[0], "3-8": bins[1], "9-16": bins[2],
        "17-32": bins[3], "33-63": bins[4], "64+": bins[5],
    })
}

#[test]
#[ignore = "handoff 0072 temporal visual review; run only by the guarded packet runner"]
fn viewport() {
    let mut report = Report::open("viewport", 26, 0);
    let stage = Stage::new();
    let mut input = stage.input();
    let camera = node(&input, &stage.camera);
    let renderer = open_renderer(false);
    let output = Output::new(&renderer);
    let mut off = plain(&renderer, &input, &stage.camera);
    let still_off = shoot_viewport(
        &mut report,
        &renderer,
        &mut off,
        &input,
        &output,
        "off-still",
    );
    drop(off);
    // One owner: two still jitter cycles, then a lateral pan that carries content
    // across the viewport border on every frame.
    let mut on = temporal(&renderer, &input, &stage.camera);
    let mut shots = Vec::new();
    for k in 1..=24_u32 {
        if k > 16 {
            input.nodes[camera].world = camera_world(k - 16);
        }
        let name = format!("on-{k:02}");
        shots.push(shoot_viewport(
            &mut report,
            &renderer,
            &mut on,
            &input,
            &output,
            &name,
        ));
    }
    drop(on);
    let mut off = plain(&renderer, &input, &stage.camera);
    let pan_off = shoot_viewport(&mut report, &renderer, &mut off, &input, &output, "off-pan");
    drop(off);
    let still_edge = edges(&still_off.pixels);
    let pan_edge = edges(&pan_off.pixels);
    report.png("viewport-on-f24.png", &shots[23].pixels);
    report.png("viewport-off-f24.png", &pan_off.pixels);
    let internal = shots[0].meta().map(|m| m.internal);
    let outside = shots
        .iter()
        .map(|s| outside_delta(&s.pixels, &pan_off.pixels))
        .max()
        .unwrap_or(u8::MAX)
        .max(outside_delta(&still_off.pixels, &pan_off.pixels));
    let checks = [
        (
            "viewport_run_uninterrupted",
            internal.is_some_and(|size| continuous(&shots, size)),
        ),
        (
            "references_have_no_temporal_state",
            still_off.temporal.is_none() && pan_off.temporal.is_none(),
        ),
        ("outside_viewport_matches_reference", outside == 0),
        ("frames_match_plan", report.frames == 26),
    ];
    let body = json!({
        "renderer": describe(&renderer),
        "viewport_rect": VIEWPORT_RECT,
        "internal_size": internal,
        "observations": {
            "still_consecutive": consecutive(&shots[..16], &still_edge),
            "still_same_phase_f08_vs_f16": split(&shots[7].pixels, &shots[15].pixels, &still_edge),
            "still_f16_vs_off": split(&still_off.pixels, &shots[15].pixels, &still_edge),
            "still_f16_border_vs_off": border_split(&still_off.pixels, &shots[15].pixels, &still_edge),
            "still_activity_frames_9_16": activity_bins(&activity(&shots[8..16])),
            "pan_consecutive": consecutive(&shots[16..], &pan_edge),
            "pan_f24_vs_off": split(&pan_off.pixels, &shots[23].pixels, &pan_edge),
            "pan_f24_border_vs_off": border_split(&pan_off.pixels, &shots[23].pixels, &pan_edge),
            "outside_viewport_max_delta": outside,
            "activity_map": "not retained; still frames 9-16 bands above",
        },
    });
    assert!(
        report.finish(body, &checks),
        "viewport failed; see report.json"
    );
}
