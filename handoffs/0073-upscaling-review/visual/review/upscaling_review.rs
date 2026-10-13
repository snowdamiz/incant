//! Packet-local output-resolution upscaling appearance review for handoff
//! 0073; not production code. It is committed only under
//! handoffs/0073-upscaling-review/visual/review/ and copied into the granted
//! child checkout as crates/incant_render/tests/upscaling_review.rs by
//! visual/tools/build.py.
//!
//! Every frame is one 960x540 Rgba8UnormSrgb `RetainedScene::submit` (the
//! native host path), read back in full and hashed. Three variants share one
//! renderer and the same physical output:
//! - FULL: the ordinary output-size control, unjittered;
//! - DRS: the same scene at a fixed internal scale through the existing
//!   nearest-texel expansion, unjittered;
//! - UP: that internal scale plus explicit output-resolution temporal
//!   upscaling with the fixed REVIEW policy and its 8-phase jitter.
//!
//! Contract checks decide each case. Image metrics are observations only;
//! appearance is judged by inspecting the retained PNGs, never inferred from a
//! numeric pass. No window, presentation, desktop capture, timing or
//! performance measurement. Output goes only to the new directory named by
//! INCANT_0073_REVIEW_OUT.
#![allow(dead_code)] // The shared fixture modules carry helpers these cases do not call.

mod materials;
mod support;

use glam::{DMat4, DQuat, DVec3, Vec3};
use incant_assets::{cook_gltf, cook_texture};
use incant_doc::{
    Asset, AssetImportSettings, Entity, MeshRenderer, SpriteMode, SpriteRenderer, SpriteSampling,
    TextureUsage, Transform,
};
use incant_render::{
    Renderer, RetainedScene, RetainedTarget, SceneInput, UiLayer, Viewport,
    resolution::{ResolutionConfig, ResolutionScale},
    upscaling::{MAX_UPSCALE_TRANSITION_BYTES, UpscaleConfig, UpscaleFrame},
    wgpu,
};
use incant_render_authoring::project_input;
use incant_render_data::{TextInput, TextNode, UiInput, UiRole, UiWidget};
use incant_text::{AtlasConfig, FontSource, GlyphAtlas, TextEngine, TextStyle, Wrap};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SIZE: [u32; 2] = [960, 540];
const W: usize = 960;
const H: usize = 540;
const OUTPUT_ENV: &str = "INCANT_0073_REVIEW_OUT";
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
/// The text route's mixed-script control: a Latin ligature, an Arabic run,
/// CJK fallback and a Hebrew letter no packaged font covers.
const MIXED: &str = "ffi سلام 中文 ש";
/// Fixed policy for every UP variant. There is no quality preset; these values
/// match 0072's fixed-resolution review so the two reviews share one policy.
const REVIEW: UpscaleConfig = UpscaleConfig {
    history_weight: 0.9,
    relative_depth: 0.05,
    absolute_depth: 0.02,
    reactive_threshold: 4.,
    reactive_strength: 0.5,
};
/// Main internal scale in sixteenths: 480x270 inside a 960x540 output.
const HALF: u8 = 8;
const CAMERA_AT: [f64; 3] = [0., 2.4, 9.5];
const CAMERA_PITCH: f64 = -0.16;
const FOV_DEGREES: f64 = 50.;
const NEAR: f64 = 0.1;
const FAR: f64 = 120.;
const MOVER_AT: [f64; 3] = [2.02, 1.05, 1.2];
/// Display surround where the scene has no coverage (output.rs), sRGB codes.
const BACKDROP: [u8; 3] = [20, 21, 25];
const FULL_RECT: [f32; 4] = [0., 0., 960., 540.];
/// Fractional, offset editor-style viewport; every admitted DRS scale of it is exact.
const VIEWPORT: Viewport = Viewport {
    rect: [37.5, 21.25, 885., 497.5],
    corner_radii: [12.; 4],
    canvas_srgb: [64, 80, 96],
};
/// Physical UI (logical viewport 960x540, so one logical pixel per pixel).
const UI_BUTTON: [f32; 4] = [24., 456., 236., 516.];
const UI_LABEL: [f32; 4] = [252., 488., 600., 520.];
/// Opaque button interior `[left, top, right, bottom)`: bounds inset past the
/// 4 px radius, the border and edge coverage.
const UI_INTERIOR: [usize; 4] = [30, 462, 230, 510];
const EDGE_THRESHOLD: u8 = 24;
/// Figures are 4x4 grids of 240x135 tiles (80x45 output pixels at 3x, or 120x67
/// at 2x, nearest-neighbour). Each tile's last row and column are a separator.
const TILE: [usize; 2] = [240, 135];
const SEPARATOR: [u8; 4] = [8, 8, 10, 255];
/// World anchors of the figure crops, projected with the authored camera.
const WIRES: [f64; 3] = [-3., 3.9, -3.];
const CARD_EDGE: [f64; 3] = [-2.55, 0.9, 4.];
const SIGN: [f64; 3] = [3.4, 1.8, 0.];
const SPRITE: [f64; 3] = [0.05, 0.5, 5.6];
const TEXT_AG: [f64; 3] = [-0.2, 1.45, 4.6];
const MOVER_PATH: [f64; 3] = [2.68, 1.05, 1.2];
const FENCE: [f64; 3] = [-1.4, 0.6, -10.];
/// Horizontal wires against the backdrop (centre height, half thickness): about
/// 1.6, 0.8 and 0.4 output pixels thick at full size.
const WIRE_HEIGHTS: [f32; 3] = [4.3, 3.9, 3.5];
const WIRE_HALVES: [f32; 3] = [0.016, 0.008, 0.004];
/// World x span of the presence profile, clear of the mast and the diagonal.
const WIRE_SPAN: [f64; 2] = [-6.6, -5.2];

// ------------------------------------------------------------------ receipts

/// Append-only events plus one final report, in a directory that must be new.
/// The frame limit is hard: exceeding it panics before the submission.
struct Report {
    case: &'static str,
    root: PathBuf,
    events: File,
    frame_limit: u32,
    frames: u32,
    pngs: Vec<Value>,
}
impl Report {
    fn open(case: &'static str, frame_limit: u32) -> Self {
        let root = PathBuf::from(
            std::env::var_os(OUTPUT_ENV)
                .expect("INCANT_0073_REVIEW_OUT must name a new output directory"),
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
            frames: 0,
            pngs: Vec::new(),
        };
        report.event(json!({"event": "open", "case": case, "frame_limit": frame_limit}));
        report
    }
    fn event(&mut self, value: Value) {
        writeln!(self.events, "{value}").unwrap();
        self.events.flush().unwrap();
    }
    /// Charges one queue submission that renders into a review target (scene
    /// or UI) before it is attempted, so refusals and failures count too.
    fn charge(&mut self, name: &str, kind: &str) {
        self.frames += 1;
        assert!(self.frames <= self.frame_limit, "frame budget exceeded");
        let index = self.frames;
        self.event(json!({"event": "charge", "name": name, "kind": kind, "index": index}));
    }
    fn png(&mut self, name: &str, pixels: &[u8]) {
        assert_eq!(pixels.len(), W * H * 4, "review PNGs are 960x540 RGBA");
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
        object.insert("pngs".into(), json!(self.pngs));
        object.insert("frames_charged".into(), json!(self.frames));
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
fn policy() -> Value {
    json!({
        "history_weight": REVIEW.history_weight,
        "relative_depth": REVIEW.relative_depth,
        "absolute_depth": REVIEW.absolute_depth,
        "reactive_threshold": REVIEW.reactive_threshold,
        "reactive_strength": REVIEW.reactive_strength,
    })
}

// ------------------------------------------------------------------ renderers

fn describe(renderer: &Renderer) -> Value {
    let info = renderer.adapter.get_info();
    json!({
        "name": info.name,
        "vendor": info.vendor,
        "device": info.device,
        "device_type": format!("{:?}", info.device_type),
        "driver": info.driver,
        "driver_info": info.driver_info,
        "backend": format!("{:?}", info.backend),
        "max_texture_dimension_2d": renderer.device.limits().max_texture_dimension_2d,
    })
}
/// Headless renderer on the required backend with the startup-only opt-in.
fn upscaling_renderer() -> Renderer {
    let mut renderer = support::checked(Renderer::headless().expect("headless renderer"));
    pollster::block_on(renderer.enable_temporal_upscaling()).expect("enable temporal upscaling");
    renderer
}
/// A fixed internal scale: equal bounds and no telemetry, so it never moves.
fn fixed(sixteenths: u8) -> ResolutionConfig {
    let scale = ResolutionScale::new(sixteenths).unwrap();
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
/// Internal attachment size of the full output at a scale (resolution/plan.rs).
fn internal(sixteenths: u8) -> [u32; 2] {
    SIZE.map(|v| (v as f32 * f32::from(sixteenths) / 16.).ceil() as u32)
}
fn variant(
    renderer: &Renderer,
    input: &SceneInput,
    camera: &str,
    scale: Option<u8>,
    upscale: bool,
) -> RetainedScene {
    let mut scene = renderer
        .prepare_retained(input, camera)
        .expect("prepare retained scene");
    if let Some(sixteenths) = scale {
        scene
            .set_dynamic_resolution(Some(fixed(sixteenths)))
            .expect("fixed internal scale");
    }
    if upscale {
        scene
            .enable_temporal_upscaling(renderer, REVIEW)
            .expect("output-resolution upscaling owner");
    }
    scene
}

// ------------------------------------------------------------------ frames

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Full,
    Drs,
    Up,
}
struct Shot {
    name: String,
    variant: Variant,
    pixels: Vec<u8>,
    frame: Option<UpscaleFrame>,
    internal: Option<[u32; 2]>,
}
fn review_target(renderer: &Renderer) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("0073 review target"),
        size: wgpu::Extent3d {
            width: SIZE[0],
            height: SIZE[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    (texture, view)
}
/// Full readback after the scene (and optional UI) submissions on one queue.
fn read(renderer: &Renderer, texture: &wgpu::Texture) -> Vec<u8> {
    let [width, height] = SIZE;
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("0073 review readback"),
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
/// One charged scene submission into a new 960x540 target, optionally followed
/// by the physical UI on the same target, then a full readback.
#[allow(clippy::too_many_arguments)]
fn shoot(
    report: &mut Report,
    renderer: &Renderer,
    scene: &mut RetainedScene,
    input: &SceneInput,
    variant: Variant,
    viewport: Option<Viewport>,
    ui: Option<(&mut UiLayer, &Arc<UiInput>)>,
    name: &str,
) -> Shot {
    let (texture, view) = review_target(renderer);
    let output = || RetainedTarget {
        view: &view,
        format: FORMAT,
        width: SIZE[0],
        height: SIZE[1],
        viewport,
    };
    report.charge(name, "scene");
    scene
        .submit(renderer, input, output())
        .expect("retained submit");
    let with_ui = ui.is_some();
    if let Some((layer, ui)) = ui {
        report.charge(name, "ui");
        let stage = layer.stage(renderer, ui).expect("UI stage");
        layer.submit(renderer, stage, &output()).expect("UI submit");
    }
    let pixels = read(renderer, &texture);
    let frame = scene.upscale_frame();
    let internal = scene.resolution_frame().map(|r| r.internal_size());
    let motion = frame.as_ref().map(|f| f.motion());
    report.event(json!({
        "event": "shot",
        "name": name,
        "variant": format!("{variant:?}"),
        "ui": with_ui,
        "rgba_sha256": hex(&pixels),
        "phase": motion.and_then(|m| m.jitter_phase()),
        "history": motion.map(|m| m.has_previous()),
        "sequence": motion.map(|m| m.sequence()),
        "epoch": motion.map(|m| m.epoch()),
        "upscale_internal": motion.map(|m| m.internal_size()),
        "upscale_output": frame.as_ref().map(|f| f.output_size()),
        "resolution_internal": internal,
    }));
    Shot {
        name: name.into(),
        variant,
        pixels,
        frame,
        internal,
    }
}
/// Expected submitted-owner state of one UP frame. `epoch` is relative to the
/// first frame of the sequence.
struct Expect {
    phase: u32,
    history: bool,
    epoch: u64,
    internal: [u32; 2],
}
/// One owner throughout, consecutive sequences, and the expected phase,
/// history, epoch and internal size at the physical output size.
fn matches(shots: &[&Shot], expected: &[Expect]) -> bool {
    let Some(first) = shots.first().and_then(|s| s.frame.as_ref()) else {
        return false;
    };
    let first = first.motion();
    shots.len() == expected.len()
        && shots
            .iter()
            .zip(expected)
            .enumerate()
            .all(|(k, (shot, e))| {
                shot.frame.as_ref().is_some_and(|f| {
                    let m = f.motion();
                    m.same_owner(first)
                        && m.sequence() == first.sequence() + k as u64
                        && m.epoch() == first.epoch() + e.epoch
                        && m.has_previous() == e.history
                        && m.jitter_phase() == Some(e.phase)
                        && !m.bypassed()
                        && m.internal_size() == e.internal
                        && f.output_size() == SIZE
                })
            })
}
fn frames_json(shots: &[&Shot]) -> Value {
    json!(
        shots
            .iter()
            .map(|s| {
                let motion = s.frame.as_ref().map(|f| f.motion());
                json!({
                    "name": s.name,
                    "variant": format!("{:?}", s.variant),
                    "rgba_sha256": hex(&s.pixels),
                    "phase": motion.and_then(|m| m.jitter_phase()),
                    "history": motion.map(|m| m.has_previous()),
                    "internal": s.internal,
                })
            })
            .collect::<Vec<_>>()
    )
}

// ------------------------------------------------------------------ pixels

fn index(x: usize, y: usize) -> usize {
    (y * W + x) * 4
}
/// Largest RGB code difference between pixel byte offsets `i` of `a` and `j` of `b`.
fn spread(a: &[u8], i: usize, b: &[u8], j: usize) -> u8 {
    (0..3)
        .map(|c| a[i + c].abs_diff(b[j + c]))
        .max()
        .unwrap_or(0)
}
fn distinct(pixels: &[u8]) -> usize {
    pixels.chunks_exact(4).collect::<HashSet<_>>().len()
}
fn in_ui(x: usize, y: usize) -> bool {
    let (px, py) = (x as f32, y as f32);
    [UI_BUTTON, UI_LABEL]
        .iter()
        .any(|r| px >= r[0] - 2. && px < r[2] + 2. && py >= r[1] - 2. && py < r[3] + 2.)
}
fn ui_interior(pixels: &[u8]) -> Vec<u8> {
    let [left, top, right, bottom] = UI_INTERIOR;
    (top..bottom)
        .flat_map(|y| pixels[index(left, y)..index(right, y)].to_vec())
        .collect()
}
/// Pixel centres inside `rect` shrunk by `inset` on every side.
fn inside(rect: [f32; 4], inset: f32) -> impl Fn(usize, usize) -> bool {
    move |x, y| {
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        px >= rect[0] + inset
            && px < rect[0] + rect[2] - inset
            && py >= rect[1] + inset
            && py < rect[1] + rect[3] - inset
    }
}
/// Control pixels whose colour differs from a 4-neighbour by more than 24 codes:
/// silhouettes, thin detail, shadow and texture edges.
fn edges(reference: &[u8]) -> Vec<bool> {
    let mut mask = vec![false; W * H];
    for y in 0..H {
        for x in 0..W {
            let i = index(x, y);
            let neighbours = [
                (x > 0).then(|| index(x - 1, y)),
                (x + 1 < W).then(|| index(x + 1, y)),
                (y > 0).then(|| index(x, y - 1)),
                (y + 1 < H).then(|| index(x, y + 1)),
            ];
            mask[y * W + x] = neighbours
                .into_iter()
                .flatten()
                .any(|j| spread(reference, i, reference, j) > EDGE_THRESHOLD);
        }
    }
    mask
}
/// Mean RGB code difference, its 99th percentile and the changed fraction,
/// split by the control's edge mask over the kept pixels. Observation only.
fn split(a: &[u8], b: &[u8], edge: &[bool], keep: &dyn Fn(usize, usize) -> bool) -> Value {
    let mut groups: [Vec<u8>; 2] = [Vec::new(), Vec::new()];
    let mut sums = [0f64; 2];
    for y in 0..H {
        for x in 0..W {
            if !keep(x, y) {
                continue;
            }
            let i = index(x, y);
            let k = usize::from(edge[y * W + x]);
            let total: u32 = (0..3).map(|c| u32::from(a[i + c].abs_diff(b[i + c]))).sum();
            sums[k] += f64::from(total) / 3.;
            groups[k].push(spread(a, i, b, i));
        }
    }
    let summary = |k: usize| {
        let mut values = groups[k].clone();
        values.sort_unstable();
        let count = values.len();
        json!({
            "pixels": count,
            "mean_abs": sums[k] / count.max(1) as f64,
            "p99_max_channel": values.get(count * 99 / 100).copied(),
            "changed_fraction": values.iter().filter(|&&v| v > 0).count() as f64 / count.max(1) as f64,
        })
    };
    json!({"interior": summary(0), "edge": summary(1)})
}
/// Per-pixel largest per-channel range over the frames (temporal activity).
fn activity(frames: &[&[u8]]) -> Vec<u8> {
    (0..W * H)
        .map(|p| {
            (0..3)
                .map(|c| {
                    let (lo, hi) = frames.iter().fold((u8::MAX, 0u8), |(lo, hi), f| {
                        (lo.min(f[p * 4 + c]), hi.max(f[p * 4 + c]))
                    });
                    hi.saturating_sub(lo)
                })
                .max()
                .unwrap_or(0)
        })
        .collect()
}
fn activity_stats(map: &[u8], edge: &[bool], keep: &dyn Fn(usize, usize) -> bool) -> Value {
    let summary = |want: bool| {
        let values: Vec<u8> = (0..W * H)
            .filter(|&p| keep(p % W, p / W) && edge[p] == want)
            .map(|p| map[p])
            .collect();
        let count = values.len().max(1) as f64;
        let above = |t: u8| values.iter().filter(|&&v| v > t).count() as f64 / count;
        json!({
            "pixels": values.len(),
            "mean": values.iter().map(|&v| f64::from(v)).sum::<f64>() / count,
            "max": values.iter().copied().max(),
            "fraction_above_2": above(2),
            "fraction_above_8": above(8),
            "fraction_above_32": above(32),
        })
    };
    json!({"interior": summary(false), "edge": summary(true)})
}
/// Rounded per-channel mean of the frames.
fn mean_frame(frames: &[&[u8]]) -> Vec<u8> {
    let n = frames.len() as u32;
    (0..W * H * 4)
        .map(|i| {
            let total: u32 = frames.iter().map(|f| u32::from(f[i])).sum();
            ((total + n / 2) / n.max(1)) as u8
        })
        .collect()
}
/// Per-pixel largest RGB code difference.
fn abs_map(a: &[u8], b: &[u8]) -> Vec<u8> {
    (0..W * H).map(|p| spread(a, p * 4, b, p * 4)).collect()
}
fn region_mean(rect: [usize; 4], value: impl Fn(usize, usize) -> f64) -> f64 {
    let [x0, y0, w, h] = rect;
    let mut total = 0.;
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            total += value(x, y);
        }
    }
    total / (w * h).max(1) as f64
}

// ------------------------------------------------------------------ figures

/// Nearest-neighbour enlargement of an output-pixel rectangle; no filtering.
fn crop_zoom(frame: &[u8], rect: [usize; 4], zoom: usize) -> (Vec<u8>, [usize; 2]) {
    let [x0, y0, w, h] = rect;
    let mut out = Vec::with_capacity(w * h * zoom * zoom * 4);
    for y in 0..h * zoom {
        for x in 0..w * zoom {
            let i = index(x0 + x / zoom, y0 + y / zoom);
            out.extend_from_slice(&frame[i..i + 4]);
        }
    }
    (out, [w * zoom, h * zoom])
}
/// Grey RGBA from a per-pixel map, amplified by `gain` and clamped.
fn gray(values: &[u8], gain: u16) -> Vec<u8> {
    values
        .iter()
        .flat_map(|&v| {
            let g = (u16::from(v) * gain).min(255) as u8;
            [g, g, g, 255]
        })
        .collect()
}
/// Crop of a per-pixel map (W*H values), amplified to grey and enlarged.
fn map_tile(map: &[u8], rect: [usize; 4], zoom: usize, gain: u16) -> (Vec<u8>, [usize; 2]) {
    crop_zoom(&gray(map, gain), rect, zoom)
}
struct Figure(Vec<u8>);
impl Figure {
    fn new() -> Self {
        Self(SEPARATOR.repeat(W * H))
    }
    /// Places a tile in slot 0..16 (row-major 4x4), keeping the separator.
    fn put(&mut self, slot: usize, tile: (Vec<u8>, [usize; 2])) {
        let (pixels, [tw, th]) = tile;
        let (column, row) = (slot % 4, slot / 4);
        for y in 0..th.min(TILE[1] - 1) {
            for x in 0..tw.min(TILE[0] - 1) {
                let source = (y * tw + x) * 4;
                let target = index(column * TILE[0] + x, row * TILE[1] + y);
                self.0[target..target + 4].copy_from_slice(&pixels[source..source + 4]);
            }
        }
    }
}

// ------------------------------------------------------------------ camera

fn camera_world(step: u32) -> DMat4 {
    // The camera pans 0.05 m per frame after frame 14 of the motion case.
    let pan = f64::from(step.saturating_sub(14)) * 0.05;
    DMat4::from_rotation_translation(
        DQuat::from_rotation_x(CAMERA_PITCH),
        DVec3::new(CAMERA_AT[0] + pan, CAMERA_AT[1], CAMERA_AT[2]),
    )
}
/// Static for 8 frames, then 0.12 m (about 8 output pixels) per frame to the
/// right: it emerges from behind the sphere and crosses the textured sign.
fn mover_world(step: u32) -> DMat4 {
    let moved = f64::from(step.saturating_sub(8)) * 0.12;
    DMat4::from_translation(DVec3::new(MOVER_AT[0] + moved, MOVER_AT[1], MOVER_AT[2]))
}
/// Pixel position of a world point: the renderer's perspective_rh with the
/// viewport aspect, top-left origin (camera.rs, wgpu viewport mapping).
fn project(camera: DMat4, rect: [f32; 4], point: [f64; 3]) -> [f64; 2] {
    let r = rect.map(f64::from);
    let projection = DMat4::perspective_rh(FOV_DEGREES.to_radians(), r[2] / r[3], NEAR, FAR);
    let clip = projection * camera.inverse() * DVec3::from_array(point).extend(1.);
    let ndc = clip.truncate() / clip.w;
    [
        r[0] + (ndc.x * 0.5 + 0.5) * r[2],
        r[1] + (0.5 - ndc.y * 0.5) * r[3],
    ]
}
fn crop_at(center: [f64; 2], size: [usize; 2]) -> [usize; 4] {
    let [w, h] = size;
    let x = (center[0] - w as f64 / 2.)
        .round()
        .clamp(0., (W - w) as f64) as usize;
    let y = (center[1] - h as f64 / 2.)
        .round()
        .clamp(0., (H - h) as f64) as usize;
    [x, y, w, h]
}
/// Mean over the wire's left span of the strongest deviation from the backdrop
/// within 3 rows of its projected centre line (0 = absent).
fn presence(pixels: &[u8], height: f32) -> f64 {
    let camera = camera_world(1);
    let left = project(camera, FULL_RECT, [WIRE_SPAN[0], f64::from(height), -3.]);
    let right = project(camera, FULL_RECT, [WIRE_SPAN[1], f64::from(height), -3.]);
    let row = left[1].round() as i64;
    let (x0, x1) = (
        (left[0].ceil() as usize).min(W),
        (right[0].floor() as usize).min(W),
    );
    let mut total = 0.;
    for x in x0..x1 {
        let mut best = 0u8;
        for y in (row - 3).max(0)..=(row + 3).min(H as i64 - 1) {
            let i = index(x, y as usize);
            let deviation = (0..3)
                .map(|c| pixels[i + c].abs_diff(BACKDROP[c]))
                .max()
                .unwrap_or(0);
            best = best.max(deviation);
        }
        total += f64::from(best);
    }
    total / x1.saturating_sub(x0).max(1) as f64
}

// ------------------------------------------------------------------ fixtures

/// Shadow look-dev builders from the compiled 0068 review helper, plus UVs.
#[derive(Default)]
struct Mesh {
    positions: Vec<f32>,
    normals: Vec<f32>,
    uvs: Vec<f32>,
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
/// Ground to the horizon, a cube, a sphere, 16 thin posts receding to 56 m,
/// and an antenna whose three wires and diagonal sit against the backdrop.
fn stage_mesh() -> Mesh {
    let mut m = Mesh::default();
    m.tilted_quad(
        Vec3::new(0., 0., -50.),
        Vec3::new(40., 0., 0.),
        Vec3::new(0., 0., -60.),
    );
    m.cuboid(Vec3::new(-2.6, 0.5, 2.6), Vec3::splat(0.5));
    m.sphere(Vec3::new(1.3, 0.85, 1.8), 0.85, 24, 48);
    for k in 0..16 {
        m.cuboid(
            Vec3::new(-1.4, 0.6, -1.5 - 3. * k as f32),
            Vec3::new(0.06, 0.6, 0.06),
        );
    }
    m.cuboid(Vec3::new(-4.4, 2.3, -3.), Vec3::new(0.05, 2.3, 0.05));
    for (height, half) in WIRE_HEIGHTS.into_iter().zip(WIRE_HALVES) {
        m.cuboid(Vec3::new(-4.4, height, -3.), Vec3::new(2.5, half, half));
    }
    // A camera-facing 0.012 m strip from (-6.9, 2.7) to (-1.9, 4.5).
    m.tilted_quad(
        Vec3::new(-4.4, 3.6, -3.),
        Vec3::new(2.5, 0.9, 0.),
        Vec3::new(-1.8, 5., 0.).normalize() * 0.006,
    );
    m
}
/// A 2.2 m camera-facing square with UVs; 128 texels span about 134 pixels.
fn sign_mesh() -> Mesh {
    let center = Vec3::new(3.4, 1.25, 0.);
    let h = 1.1_f32;
    let corners = [
        center + Vec3::new(-h, -h, 0.),
        center + Vec3::new(h, -h, 0.),
        center + Vec3::new(h, h, 0.),
        center + Vec3::new(-h, h, 0.),
    ];
    let uv: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];
    let mut m = Mesh::default();
    for i in [0, 1, 2, 0, 2, 3] {
        m.positions.extend(corners[i].to_array());
        m.normals.extend(Vec3::Z.to_array());
        m.uvs.extend(uv[i]);
    }
    m
}
/// Semi-transparent BLEND card in front of the cube's right half.
fn card_mesh() -> Mesh {
    let mut m = Mesh::default();
    m.tilted_quad(
        Vec3::new(-1.7, 0.95, 4.4),
        Vec3::new(0.55, 0., 0.),
        Vec3::new(0., 0.38, 0.),
    );
    m
}
fn mover_mesh() -> Mesh {
    let mut m = Mesh::default();
    m.cuboid(Vec3::ZERO, Vec3::splat(0.32));
    m
}
/// 128x128 look-dev chart: 1-texel lines (top left), a 2-texel checker (top
/// right), rings whose spacing falls below a texel (bottom left) and a 4-texel
/// checker crossed by 1-texel diagonals (bottom right).
fn sign_texture() -> Vec<u8> {
    let mut pixels: Vec<u8> = Vec::with_capacity(128 * 128 * 4);
    for y in 0..128_i32 {
        for x in 0..128_i32 {
            let on = match (x / 64, y / 64) {
                (0, 0) => x % 2 == 0,
                (1, 0) => (x / 2 + y / 2) % 2 == 0,
                (0, _) => ((x - 32).pow(2) + (y - 96).pow(2)) / 24 % 2 == 0,
                _ => (x / 4 + y / 4) % 2 == 0 || (x - y).rem_euclid(16) == 0,
            };
            let v: u8 = if on { 232 } else { 28 };
            pixels.extend([v, v, v, 255]);
        }
    }
    pixels
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
/// One glTF primitive: positions, normals and optional UVs in one buffer.
fn gltf(mesh: &Mesh, bin: &str, material: Value) -> (Vec<u8>, Value) {
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
    if !mesh.uvs.is_empty() {
        bytes.extend(mesh.uvs.iter().flat_map(|v| v.to_le_bytes()));
        views.push(json!({"buffer":0,"byteOffset":2 * half,"byteLength":count * 8}));
        accessors.push(json!({"bufferView":2,"componentType":5126,"count":count,"type":"VEC2"}));
        attributes["TEXCOORD_0"] = json!(2);
    }
    let document = json!({
        "asset":{"version":"2.0"}, "buffers":[{"uri":bin,"byteLength":bytes.len()}],
        "bufferViews":views, "accessors":accessors,
        "meshes":[{"primitives":[{"attributes":attributes,"material":0}]}],
        "materials":[material], "nodes":[{"mesh":0}], "scenes":[{"nodes":[0]}], "scene":0
    });
    (bytes, document)
}
/// Replaces the fixture's own model with the static stage and recooks it.
fn write_stage(f: &mut materials::Fixture) {
    let material = json!({"pbrMetallicRoughness":{"baseColorFactor":[0.62,0.62,0.6,1],"metallicFactor":0,"roughnessFactor":0.85}});
    let (bytes, document) = gltf(&stage_mesh(), "quad.bin", material);
    std::fs::write(f.root.path().join("quad.bin"), &bytes).unwrap();
    std::fs::write(
        f.root.path().join("quad.gltf"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    f.cook();
}
/// Cooks one more model beside the fixture's own, optionally with a base
/// colour texture sampled trilinearly from its full mip chain.
fn add_model(
    f: &mut materials::Fixture,
    stem: &str,
    mesh: &Mesh,
    material: Value,
    texture: Option<(&[u8], u32, u32)>,
) -> String {
    let root = f.root.path().to_path_buf();
    let (bytes, mut document) = gltf(mesh, &format!("{stem}.bin"), material);
    std::fs::write(root.join(format!("{stem}.bin")), &bytes).unwrap();
    if let Some((pixels, width, height)) = texture {
        materials::write_png(&root.join(format!("{stem}.png")), pixels, width, height);
        document["images"] = json!([{"uri": format!("{stem}.png")}]);
        document["textures"] = json!([{"source":0,"sampler":0}]);
        document["samplers"] =
            json!([{"magFilter":9729,"minFilter":9987,"wrapS":33071,"wrapT":33071}]);
    }
    let file = format!("{stem}.gltf");
    std::fs::write(root.join(&file), serde_json::to_vec(&document).unwrap()).unwrap();
    let cooked = cook_gltf(&root, Path::new(&file), &root.join(".incant/cache/models")).unwrap();
    let id = incant_doc::new_id();
    f.project.assets.insert(
        id.clone(),
        Asset {
            id: id.clone(),
            name: stem.into(),
            path: file,
            kind: "model".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: None,
        },
    );
    id
}
fn add_texture(f: &mut materials::Fixture, stem: &str, pixels: &[u8], size: u32) -> String {
    let file = format!("{stem}.png");
    materials::write_png(&f.root.path().join(&file), pixels, size, size);
    let cooked = cook_texture(
        f.root.path(),
        Path::new(&file),
        &f.root.path().join(".incant/cache/textures"),
        TextureUsage::Color,
    )
    .unwrap();
    let id = incant_doc::new_id();
    f.project.assets.insert(
        id.clone(),
        Asset {
            id: id.clone(),
            name: stem.into(),
            path: file,
            kind: "texture".into(),
            sha256: cooked.metadata.fingerprint,
            import_settings: Some(AssetImportSettings::Texture {
                usage: TextureUsage::Color,
            }),
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
fn mesh_entity(name: &str, asset: &str, cast_shadows: bool, transform: Transform) -> Entity {
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
fn sun_rotation(elevation_degrees: f64, azimuth_degrees: f64) -> [f64; 4] {
    (DQuat::from_rotation_y(azimuth_degrees.to_radians())
        * DQuat::from_rotation_x(-elevation_degrees.to_radians()))
    .to_array()
}
fn sprite_entity(index: usize, texture: &str, x: f64, flip_x: bool) -> Entity {
    let mut sprite = Entity::new(format!("Sprite {index}"));
    sprite.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [x, 0.5, 5.6],
            ..Default::default()
        }),
    );
    sprite.components.insert(
        "SpriteRenderer".into(),
        json!(SpriteRenderer {
            texture: texture.into(),
            region: [0u32, 0, 16, 16].into(),
            size: [0.45, 0.45],
            pivot: [0.5, 0.5],
            tint: [1.0, 1.0, 1.0, 1.0],
            flip_x,
            flip_y: false,
            layer: 0,
            order: index as i32,
            sampling: SpriteSampling::Nearest,
            mode: SpriteMode::Simple
        }),
    );
    sprite
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
/// World-space text in the current-only overlay: the mixed-script line at
/// 20 px and "Ag" at 48 px (100 logical pixels per metre).
fn text_input() -> TextInput {
    let mut engine = text_engine(&[LATIN, ARABIC, CJK]);
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 512,
        height: 256,
        max_glyphs: 512,
    })
    .unwrap();
    let nodes = [
        (1u8, MIXED, 20., [-2.0, 2.3, 4.6]),
        (2, "Ag", 48., [-0.5, 1.75, 4.6]),
    ]
    .into_iter()
    .map(|(id, text, size, [x, y, z])| {
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
            id: incant_types::StableId([id; 16]),
            world: DMat4::from_translation(DVec3::new(x, y, z)),
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
/// Physical UI drawn after the scene without jitter or scaling: an opaque
/// button and a label directly over the scene.
fn ui_input() -> Arc<UiInput> {
    let mut engine = text_engine(&[LATIN]);
    let mut atlas = GlyphAtlas::new(AtlasConfig {
        width: 512,
        height: 256,
        max_glyphs: 256,
    })
    .unwrap();
    let mut widget = |n: u8, role: UiRole, bounds: [f32; 4], text: &str, size: f32| {
        let padding: f32 = 8.;
        let clip = [
            bounds[0] + padding,
            bounds[1] + padding,
            bounds[2] - padding,
            bounds[3] - padding,
        ];
        let layout = engine
            .shape(
                text,
                TextStyle {
                    font_size: size,
                    line_height: (size * 1.25).round(),
                    width: clip[2] - clip[0],
                    wrap: Wrap::None,
                },
            )
            .unwrap();
        let glyphs: Vec<_> = (0..layout.glyphs().len())
            .map(|i| engine.cache_glyph(&layout, i, &mut atlas).unwrap())
            .collect();
        UiWidget {
            id: incant_types::StableId([n; 16]),
            role,
            disabled: false,
            focused: false,
            pressed: false,
            bounds,
            visible: bounds,
            origin: [clip[0], clip[1]],
            clip,
            layout,
            glyphs: glyphs.into(),
            edit: None,
        }
    };
    let widgets = vec![
        widget(1, UiRole::Button, UI_BUTTON, "Upscale 50%", 22.),
        widget(2, UiRole::Label, UI_LABEL, "physical UI 960x540", 18.),
    ];
    Arc::new(UiInput {
        viewport: [SIZE[0] as f32, SIZE[1] as f32],
        atlas: Arc::new(atlas),
        widgets,
    })
}

/// The look-dev stage: lit opaque geometry with thin detail, a trilinear
/// textured chart, a BLEND card, three nearest-sampled sprites, world text and,
/// for the motion case, a moving cube. Bindings never change within a case.
struct Stage {
    f: materials::Fixture,
    camera: String,
    mover: Option<String>,
}
impl Stage {
    fn new(with_mover: bool) -> Self {
        let mut f = materials::Fixture::new(json!({}));
        write_stage(&mut f);
        f.environment(&[34, 36, 44, 255].repeat(2), 2, 1, TextureUsage::Linear);
        let chart = sign_texture();
        let sign = add_model(
            &mut f,
            "sign",
            &sign_mesh(),
            json!({"pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":0.9}}),
            Some((chart.as_slice(), 128, 128)),
        );
        let card = add_model(
            &mut f,
            "card",
            &card_mesh(),
            materials::emissive([0.9, 0.35, 0.1], 0.45, "BLEND"),
            None,
        );
        let mover_asset = with_mover.then(|| {
            add_model(
                &mut f,
                "mover",
                &mover_mesh(),
                json!({"pbrMetallicRoughness":{"baseColorFactor":[0.75,0.12,0.08,1],"metallicFactor":0,"roughnessFactor":0.6}}),
                None,
            )
        });
        let badge_texture = badge();
        let badge = add_texture(&mut f, "badge", &badge_texture, 16);
        let mut camera = Entity::new("Review camera");
        camera.components.insert(
            "Camera".into(),
            json!({"fov_degrees":FOV_DEGREES,"near":NEAR,"far":FAR}),
        );
        camera.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: CAMERA_AT,
                rotation: DQuat::from_rotation_x(CAMERA_PITCH).to_array(),
                ..Default::default()
            }),
        );
        let camera = insert(&mut f, camera);
        let mut sun = Entity::new("Review sun");
        sun.components.insert(
            "DirectionalLight".into(),
            json!({"color":[1,0.96,0.9],"intensity":3.2,"shadows":{"distance":60}}),
        );
        sun.components.insert(
            "Transform".into(),
            json!(Transform {
                rotation: sun_rotation(38., 30.),
                ..Default::default()
            }),
        );
        insert(&mut f, sun);
        insert(
            &mut f,
            mesh_entity("Detail sign", &sign, false, Transform::default()),
        );
        insert(
            &mut f,
            mesh_entity("Blend card", &card, false, Transform::default()),
        );
        let mover = mover_asset.map(|asset| {
            insert(
                &mut f,
                mesh_entity(
                    "Mover",
                    &asset,
                    true,
                    Transform {
                        translation: MOVER_AT,
                        ..Default::default()
                    },
                ),
            )
        });
        for (index, (x, flip_x)) in [(-0.6, false), (0.05, true), (0.7, false)]
            .into_iter()
            .enumerate()
        {
            insert(&mut f, sprite_entity(index, &badge, x, flip_x));
        }
        f.assets.sync_project(&f.project, f.root.path()).unwrap();
        Self { f, camera, mover }
    }
    fn input(&self) -> SceneInput {
        let mut input = project_input(&self.f.project, Some(&self.f.assets)).unwrap();
        input.text = Some(text_input());
        input
    }
    /// Camera and mover pose of one frame step; nothing else changes.
    fn pose(&self, input: &mut SceneInput, step: u32) {
        let camera = node(input, &self.camera);
        input.nodes[camera].world = camera_world(step);
        if let Some(mover) = &self.mover {
            let mover = node(input, mover);
            input.nodes[mover].world = mover_world(step);
        }
    }
}
fn node(input: &SceneInput, id: &str) -> usize {
    input
        .nodes
        .iter()
        .position(|n| n.id == id)
        .expect("authored node")
}

// ------------------------------------------------------------------ cases

#[test]
#[ignore = "handoff 0073 packet review; run only by the guarded packet runner"]
fn probe() {
    let mut report = Report::open("probe", 0);
    let stage = Stage::new(false);
    let input = stage.input();
    let mut renderer = support::checked(Renderer::headless().expect("headless renderer"));
    let mut scene = renderer
        .prepare_retained(&input, &stage.camera)
        .expect("prepare retained scene");
    let before = scene
        .enable_temporal_upscaling(&renderer, REVIEW)
        .map_err(|e| e.to_string());
    let first = pollster::block_on(renderer.enable_temporal_upscaling()).map_err(|e| e.to_string());
    let again = pollster::block_on(renderer.enable_temporal_upscaling()).map_err(|e| e.to_string());
    let invalid = [
        UpscaleConfig {
            history_weight: 1.,
            ..REVIEW
        },
        UpscaleConfig {
            relative_depth: 0.2,
            ..REVIEW
        },
        UpscaleConfig {
            absolute_depth: -0.1,
            ..REVIEW
        },
        UpscaleConfig {
            reactive_threshold: 0.,
            ..REVIEW
        },
        UpscaleConfig {
            reactive_strength: f32::NAN,
            ..REVIEW
        },
    ];
    let refused: Vec<bool> = invalid.iter().map(|c| c.validate().is_err()).collect();
    let owner_refused: Vec<bool> = invalid
        .iter()
        .map(|c| scene.enable_temporal_upscaling(&renderer, *c).is_err())
        .collect();
    let accepted = scene
        .enable_temporal_upscaling(&renderer, REVIEW)
        .map_err(|e| e.to_string());
    // Logical per-owner footprint at this output and scale (upscaling/plan.rs
    // accounting), recorded for context only: not a phone budget.
    let [ow, oh] = SIZE.map(u64::from);
    let [iw, ih] = internal(HALF).map(u64::from);
    let footprint = 32 * ow * oh + 8 * iw * ih + 24 * iw * ih;
    let checks = [
        (
            "adapter_backend_metal",
            renderer.adapter.get_info().backend == wgpu::Backend::Metal,
        ),
        ("owner_refused_before_startup_opt_in", before.is_err()),
        ("startup_opt_in_succeeds", first.is_ok()),
        ("startup_opt_in_idempotent", again.is_ok()),
        ("review_policy_valid", REVIEW.validate().is_ok()),
        ("five_invalid_policies_refused", refused.iter().all(|&r| r)),
        (
            "owner_refuses_invalid_policies",
            owner_refused.iter().all(|&r| r),
        ),
        ("owner_accepts_review_policy", accepted.is_ok()),
    ];
    let body = json!({
        "adapter": describe(&renderer),
        "policy": policy(),
        "results": {
            "owner_before_opt_in": before.err(),
            "startup_opt_in": first.err(),
            "startup_opt_in_again": again.err(),
            "owner_review_policy": accepted.err(),
            "invalid_policies_refused": refused,
            "owner_invalid_policies_refused": owner_refused,
        },
        "logical_bytes": {
            "max_upscale_transition_bytes": MAX_UPSCALE_TRANSITION_BYTES,
            "owner_960x540_at_8_16_targets": footprint,
            "note": "logical attachment accounting only; not RSS, a phone budget or a measurement",
        },
    });
    assert!(
        report.finish(body, &checks),
        "probe failed; see report.json"
    );
}

/// Static stage at 960x540: FULL and DRS 8/16 controls, then 24 UP frames (three
/// jitter cycles; the UI is drawn on both controls and the last 8).
#[test]
#[ignore = "handoff 0073 packet review; run only by the guarded packet runner"]
fn stationary() {
    let mut report = Report::open("stationary", 36);
    let stage = Stage::new(false);
    let mut input = stage.input();
    stage.pose(&mut input, 1);
    let renderer = upscaling_renderer();
    let ui = ui_input();
    let mut layer = renderer.ui_layer();
    let mut full = variant(&renderer, &input, &stage.camera, None, false);
    let mut drs = variant(&renderer, &input, &stage.camera, Some(HALF), false);
    let mut up = variant(&renderer, &input, &stage.camera, Some(HALF), true);
    let full_shot = shoot(
        &mut report,
        &renderer,
        &mut full,
        &input,
        Variant::Full,
        None,
        Some((&mut layer, &ui)),
        "full",
    );
    let drs_shot = shoot(
        &mut report,
        &renderer,
        &mut drs,
        &input,
        Variant::Drs,
        None,
        Some((&mut layer, &ui)),
        "drs50",
    );
    let mut shots = Vec::new();
    for k in 1..=24u32 {
        let overlay = if k > 16 {
            Some((&mut layer, &ui))
        } else {
            None
        };
        shots.push(shoot(
            &mut report,
            &renderer,
            &mut up,
            &input,
            Variant::Up,
            None,
            overlay,
            &format!("up50-{k}"),
        ));
    }
    let camera = camera_world(1);
    let edge = edges(&full_shot.pixels);
    let scene_pixels = |x: usize, y: usize| !in_ui(x, y);
    let last: Vec<&[u8]> = shots[16..].iter().map(|s| s.pixels.as_slice()).collect();
    let moving = activity(&last);
    let mean = mean_frame(&last);
    let crop = |anchor: [f64; 3]| crop_at(project(camera, FULL_RECT, anchor), [80, 45]);
    let crops = [
        ("wires", crop(WIRES)),
        ("card_edge", crop(CARD_EDGE)),
        ("sign", crop(SIGN)),
        ("sprite", crop(SPRITE)),
        ("text", crop(TEXT_AG)),
    ];
    // Phases: wires (slots 0-7) and world text (slots 8-15) over frames 17-24.
    let mut phases = Figure::new();
    for (k, pixels) in last.iter().enumerate() {
        phases.put(k, crop_zoom(pixels, crops[0].1, 3));
        phases.put(8 + k, crop_zoom(pixels, crops[4].1, 3));
    }
    // Detail rows: wires, card edge, sign, sprite. Columns: FULL, DRS 8/16,
    // UP frame 24 (phase 7) and the last cycle's activity at 4x gain.
    let mut detail = Figure::new();
    for (row, (_, rect)) in crops[..4].iter().enumerate() {
        detail.put(4 * row, crop_zoom(&full_shot.pixels, *rect, 3));
        detail.put(4 * row + 1, crop_zoom(&drs_shot.pixels, *rect, 3));
        detail.put(4 * row + 2, crop_zoom(&shots[23].pixels, *rect, 3));
        detail.put(4 * row + 3, map_tile(&moving, *rect, 3, 4));
    }
    let regions: Map<String, Value> = crops
        .iter()
        .map(|(name, rect)| {
            let drs_error = region_mean(*rect, |x, y| {
                f64::from(spread(
                    &drs_shot.pixels,
                    index(x, y),
                    &full_shot.pixels,
                    index(x, y),
                ))
            });
            let up_error = region_mean(*rect, |x, y| {
                f64::from(spread(
                    &shots[23].pixels,
                    index(x, y),
                    &full_shot.pixels,
                    index(x, y),
                ))
            });
            let up_activity = region_mean(*rect, |x, y| f64::from(moving[y * W + x]));
            (
                (*name).to_owned(),
                json!({
                    "rect": rect,
                    "drs50_vs_full_mean_max_channel": drs_error,
                    "up50_frame24_vs_full_mean_max_channel": up_error,
                    "up50_last_cycle_activity_mean": up_activity,
                }),
            )
        })
        .collect();
    let wires: Vec<Value> = WIRE_HEIGHTS
        .into_iter()
        .zip(WIRE_HALVES)
        .map(|(height, half)| {
            json!({
                "height_m": height,
                "thickness_m": 2. * half,
                "full": presence(&full_shot.pixels, height),
                "drs50": presence(&drs_shot.pixels, height),
                "up50_phases_0_to_7": last.iter().map(|p| presence(p, height)).collect::<Vec<_>>(),
            })
        })
        .collect();
    let expected: Vec<Expect> = (0..24)
        .map(|k| Expect {
            phase: k % 8,
            history: k > 0,
            epoch: 0,
            internal: internal(HALF),
        })
        .collect();
    let up_shots: Vec<&Shot> = shots.iter().collect();
    let with_ui: Vec<&Shot> = [&full_shot, &drs_shot]
        .into_iter()
        .chain(shots[16..].iter())
        .collect();
    let reference_ui = ui_interior(&full_shot.pixels);
    let all: Vec<&Shot> = [&full_shot, &drs_shot]
        .into_iter()
        .chain(shots.iter())
        .collect();
    let checks = [
        (
            "full_control_has_no_upscale_or_resolution_owner",
            full_shot.frame.is_none() && full_shot.internal.is_none(),
        ),
        (
            "drs_control_internal_480x270_without_upscaling",
            drs_shot.frame.is_none() && drs_shot.internal == Some(internal(HALF)),
        ),
        (
            "up_one_owner_phases_history_and_sizes",
            matches(&up_shots, &expected),
        ),
        (
            "ui_button_interior_identical_on_all_ten_ui_frames",
            with_ui
                .iter()
                .all(|s| ui_interior(&s.pixels) == reference_ui),
        ),
        (
            "frames_nonblank",
            all.iter().all(|s| distinct(&s.pixels) >= 64),
        ),
    ];
    report.png("stationary-full.png", &full_shot.pixels);
    report.png("stationary-drs50.png", &drs_shot.pixels);
    report.png("stationary-up50.png", &shots[23].pixels);
    report.png("stationary-phases.png", &phases.0);
    report.png("stationary-detail.png", &detail.0);
    report.png("stationary-activity.png", &gray(&moving, 4));
    let body = json!({
        "policy": policy(),
        "scale_sixteenths": HALF,
        "output": SIZE,
        "internal": internal(HALF),
        "crops": crops.iter().map(|(name, rect)| json!({"name": name, "rect": rect})).collect::<Vec<_>>(),
        "figures": {
            "stationary-phases.png": "4x4 tiles, 80x45 px at 3x: slots 0-7 wires, slots 8-15 world text, UP frames 17-24 (phases 0-7)",
            "stationary-detail.png": "rows wires, card_edge, sign, sprite; columns FULL, DRS 8/16, UP frame 24, last-cycle activity x4",
            "stationary-activity.png": "per-pixel largest channel range over UP frames 17-24, x4 grey",
        },
        "observations": {
            "vs_full": {
                "drs50": split(&drs_shot.pixels, &full_shot.pixels, &edge, &scene_pixels),
                "up50_frame1_current_only": split(&shots[0].pixels, &full_shot.pixels, &edge, &scene_pixels),
                "up50_frame24": split(&shots[23].pixels, &full_shot.pixels, &edge, &scene_pixels),
                "up50_last_cycle_mean": split(&mean, &full_shot.pixels, &edge, &scene_pixels),
            },
            "up50_last_cycle_activity": activity_stats(&moving, &edge, &scene_pixels),
            "up50_consecutive_last_cycle": (17..24)
                .map(|k| split(&shots[k].pixels, &shots[k - 1].pixels, &edge, &scene_pixels))
                .collect::<Vec<_>>(),
            "regions": regions,
            "wire_presence": wires,
        },
        "frames": frames_json(&all),
    });
    assert!(
        report.finish(body, &checks),
        "stationary failed; see report.json"
    );
}

/// The stage with a moving cube: 8 static UP frames, 6 frames of object motion
/// (emerging from behind the sphere across the sign), then 6 more while the
/// camera also pans. FULL and DRS controls render frames 14 and 20 exactly.
#[test]
#[ignore = "handoff 0073 packet review; run only by the guarded packet runner"]
fn motion() {
    let mut report = Report::open("motion", 24);
    let stage = Stage::new(true);
    let mut input = stage.input();
    stage.pose(&mut input, 1);
    let renderer = upscaling_renderer();
    let mut full = variant(&renderer, &input, &stage.camera, None, false);
    let mut drs = variant(&renderer, &input, &stage.camera, Some(HALF), false);
    let mut up = variant(&renderer, &input, &stage.camera, Some(HALF), true);
    let mut shots = Vec::new();
    let mut controls = Vec::new();
    for step in 1..=20u32 {
        stage.pose(&mut input, step);
        shots.push(shoot(
            &mut report,
            &renderer,
            &mut up,
            &input,
            Variant::Up,
            None,
            None,
            &format!("up50-{step}"),
        ));
        if step == 14 || step == 20 {
            controls.push(shoot(
                &mut report,
                &renderer,
                &mut full,
                &input,
                Variant::Full,
                None,
                None,
                &format!("full-{step}"),
            ));
            controls.push(shoot(
                &mut report,
                &renderer,
                &mut drs,
                &input,
                Variant::Drs,
                None,
                None,
                &format!("drs50-{step}"),
            ));
        }
    }
    let [full14, drs14, full20, drs20] = [&controls[0], &controls[1], &controls[2], &controls[3]];
    let camera = camera_world(1);
    let mover_rect = crop_at(project(camera, FULL_RECT, MOVER_PATH), [120, 67]);
    let pan_rect = crop_at(project(camera, FULL_RECT, FENCE), [120, 67]);
    let mut detail = Figure::new();
    for (slot, k) in [7usize, 8, 10, 13].into_iter().enumerate() {
        detail.put(slot, crop_zoom(&shots[k].pixels, mover_rect, 2));
    }
    detail.put(4, crop_zoom(&full14.pixels, mover_rect, 2));
    detail.put(5, crop_zoom(&drs14.pixels, mover_rect, 2));
    detail.put(
        6,
        map_tile(
            &abs_map(&shots[13].pixels, &full14.pixels),
            mover_rect,
            2,
            4,
        ),
    );
    detail.put(
        7,
        map_tile(&abs_map(&drs14.pixels, &full14.pixels), mover_rect, 2, 4),
    );
    for (slot, k) in [13usize, 15, 17, 19].into_iter().enumerate() {
        detail.put(8 + slot, crop_zoom(&shots[k].pixels, pan_rect, 2));
    }
    detail.put(12, crop_zoom(&full20.pixels, pan_rect, 2));
    detail.put(13, crop_zoom(&drs20.pixels, pan_rect, 2));
    detail.put(
        14,
        map_tile(&abs_map(&shots[19].pixels, &full20.pixels), pan_rect, 2, 4),
    );
    detail.put(
        15,
        map_tile(&abs_map(&drs20.pixels, &full20.pixels), pan_rect, 2, 4),
    );
    let everywhere = |_: usize, _: usize| true;
    let at = |up_frame: &Shot, control: &Shot, drs_frame: &Shot| {
        let edge = edges(&control.pixels);
        let region = |a: &Shot, rect: [usize; 4]| {
            region_mean(rect, |x, y| {
                f64::from(spread(&a.pixels, index(x, y), &control.pixels, index(x, y)))
            })
        };
        json!({
            "up50_vs_full": split(&up_frame.pixels, &control.pixels, &edge, &everywhere),
            "drs50_vs_full": split(&drs_frame.pixels, &control.pixels, &edge, &everywhere),
            "mover_region": {"rect": mover_rect, "up50": region(up_frame, mover_rect), "drs50": region(drs_frame, mover_rect)},
            "pan_region": {"rect": pan_rect, "up50": region(up_frame, pan_rect), "drs50": region(drs_frame, pan_rect)},
        })
    };
    let expected: Vec<Expect> = (0..20)
        .map(|k| Expect {
            phase: k % 8,
            history: k > 0,
            epoch: 0,
            internal: internal(HALF),
        })
        .collect();
    let up_shots: Vec<&Shot> = shots.iter().collect();
    let all: Vec<&Shot> = shots.iter().chain(controls.iter()).collect();
    let checks = [
        (
            "up_history_survives_object_and_camera_motion",
            matches(&up_shots, &expected),
        ),
        (
            "controls_plain_and_drs_480x270",
            full14.frame.is_none()
                && full20.frame.is_none()
                && full14.internal.is_none()
                && drs14.frame.is_none()
                && drs20.frame.is_none()
                && drs14.internal == Some(internal(HALF))
                && drs20.internal == Some(internal(HALF)),
        ),
        (
            "frames_nonblank",
            all.iter().all(|s| distinct(&s.pixels) >= 64),
        ),
    ];
    report.png("motion-full.png", &full20.pixels);
    report.png("motion-up50.png", &shots[19].pixels);
    report.png("motion-detail.png", &detail.0);
    let body = json!({
        "policy": policy(),
        "scale_sixteenths": HALF,
        "mover_m_per_frame": 0.12,
        "camera_pan_m_per_frame_after_14": 0.05,
        "crops": {"mover": mover_rect, "pan": pan_rect},
        "figures": {
            "motion-detail.png": "4x4 tiles, 120x67 px at 2x. Row 1 mover crop UP frames 8, 9, 11, 14; row 2 FULL 14, DRS 14, |UP14-FULL14| x4, |DRS14-FULL14| x4; row 3 pan crop UP frames 14, 16, 18, 20; row 4 FULL 20, DRS 20, |UP20-FULL20| x4, |DRS20-FULL20| x4",
        },
        "observations": {
            "frame14_object_motion": at(&shots[13], full14, drs14),
            "frame20_object_and_camera_motion": at(&shots[19], full20, drs20),
        },
        "frames": frames_json(&all),
    });
    assert!(
        report.finish(body, &checks),
        "motion failed; see report.json"
    );
}

/// The stage inside a fractional offset viewport: 10 UP frames at 8/16, then
/// internal-only changes to 12, 4, 16 and 8 (3 frames each), then an explicit
/// cut followed by 5 frames at 8/16. FULL and DRS 8/16 controls use the same
/// viewport.
#[test]
#[ignore = "handoff 0073 packet review; run only by the guarded packet runner"]
fn transitions() {
    let mut report = Report::open("transitions", 29);
    let stage = Stage::new(false);
    let mut input = stage.input();
    stage.pose(&mut input, 1);
    let renderer = upscaling_renderer();
    let viewport = Some(VIEWPORT);
    let mut full = variant(&renderer, &input, &stage.camera, None, false);
    let mut drs = variant(&renderer, &input, &stage.camera, Some(HALF), false);
    let mut up = variant(&renderer, &input, &stage.camera, Some(HALF), true);
    let full_shot = shoot(
        &mut report,
        &renderer,
        &mut full,
        &input,
        Variant::Full,
        viewport,
        None,
        "full",
    );
    let drs_shot = shoot(
        &mut report,
        &renderer,
        &mut drs,
        &input,
        Variant::Drs,
        viewport,
        None,
        "drs50",
    );
    // (internal scale in sixteenths, frames, explicit cut before the segment)
    let schedule: [(u8, u32, bool); 6] = [
        (8, 10, false),
        (12, 3, false),
        (4, 3, false),
        (16, 3, false),
        (8, 3, false),
        (8, 5, true),
    ];
    let mut shots = Vec::new();
    let mut expected = Vec::new();
    let mut epoch = 0u64;
    let mut phase = 0u32;
    for (sixteenths, count, cut) in schedule {
        up.set_dynamic_resolution(Some(fixed(sixteenths)))
            .expect("fixed internal scale");
        if cut {
            up.invalidate_motion_history();
            epoch += 1;
            phase = 0;
        }
        for _ in 0..count {
            expected.push(Expect {
                phase: phase % 8,
                history: !shots.is_empty() && !(cut && phase == 0),
                epoch,
                internal: internal(sixteenths),
            });
            let name = format!("up{sixteenths}-{}", shots.len() + 1);
            shots.push(shoot(
                &mut report,
                &renderer,
                &mut up,
                &input,
                Variant::Up,
                viewport,
                None,
                &name,
            ));
            phase += 1;
        }
    }
    let rect = VIEWPORT.rect;
    let interior = inside(rect, 2.);
    let outer = inside(rect, 0.);
    let band = |x: usize, y: usize| outer(x, y) && !interior(x, y);
    // Pixel centres at least one pixel outside the rectangle are pure chrome.
    let chrome = |x: usize, y: usize| {
        let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
        px < rect[0] - 1.
            || px >= rect[0] + rect[2] + 1.
            || py < rect[1] - 1.
            || py >= rect[1] + rect[3] + 1.
    };
    let chrome_pixels: Vec<usize> = (0..W * H).filter(|&p| chrome(p % W, p / W)).collect();
    let all: Vec<&Shot> = [&full_shot, &drs_shot]
        .into_iter()
        .chain(shots.iter())
        .collect();
    let chrome_identical = all.iter().all(|s| {
        chrome_pixels
            .iter()
            .all(|&p| s.pixels[p * 4..p * 4 + 4] == full_shot.pixels[p * 4..p * 4 + 4])
    });
    let edge = edges(&full_shot.pixels);
    let camera = camera_world(1);
    let strip_rect = crop_at(project(camera, rect, SIGN), [80, 45]);
    let mut strip = Figure::new();
    for (slot, k) in [9usize, 10, 12, 13, 15, 16, 18, 19, 21, 22, 23, 24, 26]
        .into_iter()
        .enumerate()
    {
        strip.put(slot, crop_zoom(&shots[k].pixels, strip_rect, 3));
    }
    strip.put(13, crop_zoom(&full_shot.pixels, strip_rect, 3));
    strip.put(14, crop_zoom(&drs_shot.pixels, strip_rect, 3));
    strip.put(
        15,
        map_tile(
            &abs_map(&shots[21].pixels, &full_shot.pixels),
            strip_rect,
            3,
            4,
        ),
    );
    // First frame of each segment against the frame before it, beside the
    // ordinary frame-to-frame change just before the boundary.
    let boundaries: Vec<Value> = [10usize, 13, 16, 19, 22]
        .into_iter()
        .map(|k| {
            json!({
                "first_frame": k + 1,
                "scale_sixteenths": shots[k].internal.map(|s| s[0] * 16 / SIZE[0]),
                "change_at_boundary": split(&shots[k].pixels, &shots[k - 1].pixels, &edge, &interior),
                "change_before_boundary": split(&shots[k - 1].pixels, &shots[k - 2].pixels, &edge, &interior),
            })
        })
        .collect();
    let versus: Map<String, Value> = [9usize, 12, 15, 18, 21, 22, 26]
        .into_iter()
        .map(|k| {
            (
                format!("up_frame{}", k + 1),
                split(&shots[k].pixels, &full_shot.pixels, &edge, &interior),
            )
        })
        .collect();
    let up_shots: Vec<&Shot> = shots.iter().collect();
    let checks = [
        (
            "internal_scale_changes_keep_owner_and_history_then_cut_resets",
            matches(&up_shots, &expected),
        ),
        (
            "controls_plain_and_drs_480x270",
            full_shot.frame.is_none()
                && full_shot.internal.is_none()
                && drs_shot.frame.is_none()
                && drs_shot.internal == Some(internal(HALF)),
        ),
        ("chrome_outside_viewport_identical", chrome_identical),
        (
            "frames_nonblank",
            all.iter().all(|s| distinct(&s.pixels) >= 64),
        ),
    ];
    report.png("transitions-full.png", &full_shot.pixels);
    report.png("transitions-up50.png", &shots[9].pixels);
    report.png("transitions-strip.png", &strip.0);
    report.png("transitions-cut.png", &shots[22].pixels);
    let body = json!({
        "policy": policy(),
        "viewport": {"rect": rect, "corner_radii": VIEWPORT.corner_radii, "canvas_srgb": VIEWPORT.canvas_srgb},
        "schedule": schedule.iter().map(|(n, count, cut)| json!({"sixteenths": n, "frames": count, "cut_before": cut})).collect::<Vec<_>>(),
        "crops": {"strip": strip_rect},
        "figures": {
            "transitions-strip.png": "4x4 tiles, 80x45 px at 3x: UP frames 10, 11, 13, 14, 16, 17, 19, 20, 22, 23 (after the cut), 24, 25, 27, then FULL, DRS 8/16 and |UP22-FULL| x4",
        },
        "observations": {
            "boundaries": boundaries,
            "vs_full_inside_viewport": versus,
            "drs50_vs_full_inside_viewport": split(&drs_shot.pixels, &full_shot.pixels, &edge, &interior),
            "border_band_frame22_vs_full": split(&shots[21].pixels, &full_shot.pixels, &edge, &band),
            "chrome_pixels_compared": chrome_pixels.len(),
        },
        "frames": frames_json(&all),
    });
    assert!(
        report.finish(body, &checks),
        "transitions failed; see report.json"
    );
}
