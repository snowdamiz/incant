//! Packet-local owner-aware output-upscaling appearance review for handoff
//! 0073; not production code. It is committed only under
//! handoffs/0073-upscaling-review/owner_quality/review/ and is copied by root
//! into a source-identical fd5b86d0 checkout as
//! crates/incant_render/tests/upscaling_owner_review.rs (owner_quality/plan.json).
//!
//! Every frame is one 960x540 Rgba8UnormSrgb `RetainedScene::submit`, read back
//! in full by its own queue submission and map. One renderer per group carries
//! both startup opt-ins (`enable_temporal_upscaling`, then `enable_motion_owner`).
//! Variants share that renderer, the authored inputs and the chronology:
//! - FULL: the ordinary output-size control, unjittered;
//! - DRS: the same scene at a fixed internal scale (nearest-texel expansion);
//! - UP: ordinary portable upscaling. Motion history, no owner attachment,
//!   then `enable_temporal_upscaling(REVIEW)`;
//! - OWN: owner-aware portable upscaling. Motion history, then
//!   `set_motion_owner_attachment(true)`, then `enable_temporal_upscaling(REVIEW)`.
//!
//! Contract checks decide each case. The historical C7 and C9 gates were
//! defined on a linear CPU model; here their unchanged threshold values are
//! compared with tone-mapped 8-bit sRGB rendered output. Those comparisons are
//! rendered-output observations, not passes or fails of the historical gates,
//! and never decide the process outcome. Image metrics are observations only;
//! appearance is judged by inspecting the retained PNGs. No window,
//! presentation, desktop capture, timing or performance measurement. Output
//! goes only to the new directory named by INCANT_0073_OWNER_REVIEW_OUT.
#![allow(dead_code)] // The shared fixture modules carry helpers these cases do not call.

#[path = "reconstruction_diagnostic_materials.rs"]
mod materials;
use crate::test_support as support;

use crate::{
    Renderer, RetainedScene, RetainedTarget, SceneInput, UiLayer, Viewport,
    resolution::{ResolutionConfig, ResolutionScale},
    upscaling::{UpscaleConfig, UpscaleFrame},
    wgpu,
};
use glam::{DMat4, DQuat, DVec3, Vec3};
use incant_assets::{cook_gltf, cook_texture};
use incant_doc::{
    Asset, AssetImportSettings, Entity, MeshRenderer, SpriteMode, SpriteRenderer, SpriteSampling,
    TextureUsage, Transform,
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
const OUTPUT_ENV: &str = "INCANT_0073_OWNER_REVIEW_OUT";
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
/// The text route's mixed-script control: a Latin ligature, an Arabic run,
/// CJK fallback and a Hebrew letter no packaged font covers.
const MIXED: &str = "ffi سلام 中文 ש";
/// Unchanged review policy for every UP and OWN variant (no preset, no tuning).
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

// ------------------------------------------------------------------ budgets

/// Exact per-group budget. Every scene, UI and readback queue submission is
/// charged before it is attempted; each readback has exactly one map. The
/// report must consume the budget exactly (owner_quality/plan.json).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Budget {
    scenes: u32,
    ui: u32,
    readbacks: u32,
    pngs: u32,
}
impl Budget {
    const fn queue_submissions(self) -> u32 {
        self.scenes + self.ui + self.readbacks
    }
    fn json(self) -> Value {
        json!({
            "scene_submissions": self.scenes,
            "rendered_frames": self.scenes,
            "ui_submissions": self.ui,
            "readback_submissions": self.readbacks,
            "readback_maps": self.readbacks,
            "queue_submissions": self.queue_submissions(),
            "pngs": self.pngs,
        })
    }
}
/// FULL 1 + DRS 1 + UP 24 + OWN 24 scenes; UI on both controls and the last 8
/// frames of each UP variant.
const STATIONARY: Budget = Budget {
    scenes: 50,
    ui: 18,
    readbacks: 50,
    pngs: 6,
};
/// UP 20 + OWN 20 + FULL and DRS at steps 14 and 20.
const MOTION: Budget = Budget {
    scenes: 44,
    ui: 0,
    readbacks: 44,
    pngs: 4,
};
/// UP 28 + OWN 28 on the corrected schedule, fixed-16/16 UP 24 + OWN 24, FULL 1, DRS 1.
const TRANSITIONS: Budget = Budget {
    scenes: 106,
    ui: 0,
    readbacks: 106,
    pngs: 2,
};
/// UP and OWN, moving and never-occluded reference, 20 frames each; FULL moving
/// and reference at the last step.
const C7: Budget = Budget {
    scenes: 82,
    ui: 0,
    readbacks: 82,
    pngs: 2,
};
/// UP and OWN, with bar and bar-free reference, 24 frames each; FULL with bar
/// and reference at the last step.
const P1: Budget = Budget {
    scenes: 98,
    ui: 0,
    readbacks: 98,
    pngs: 1,
};

// ------------------------------------------------------------------ receipts

#[derive(Clone, Copy, Debug)]
enum Charge {
    Scene,
    Ui,
    Readback,
}
/// Append-only events plus one final report, in a directory that must be new.
/// Budgets are hard: exceeding any of them panics before the submission.
struct Report {
    case: &'static str,
    root: PathBuf,
    events: File,
    limit: Budget,
    used: Budget,
    pngs: Vec<Value>,
}
impl Report {
    fn open(case: &'static str, limit: Budget) -> Self {
        let root = PathBuf::from(
            std::env::var_os(OUTPUT_ENV)
                .expect("INCANT_0073_OWNER_REVIEW_OUT must name a new output directory"),
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
            limit,
            used: Budget::default(),
            pngs: Vec::new(),
        };
        report.event(json!({"event": "open", "case": case, "budget": limit.json()}));
        report
    }
    fn event(&mut self, value: Value) {
        writeln!(self.events, "{value}").unwrap();
        self.events.flush().unwrap();
    }
    /// Charges one explicit queue submission before it is attempted, so
    /// refusals and failures count too. A readback charge also covers its map.
    fn charge(&mut self, name: &str, kind: Charge) {
        let (used, limit) = match kind {
            Charge::Scene => (&mut self.used.scenes, self.limit.scenes),
            Charge::Ui => (&mut self.used.ui, self.limit.ui),
            Charge::Readback => (&mut self.used.readbacks, self.limit.readbacks),
        };
        *used += 1;
        let index = *used;
        assert!(index <= limit, "{kind:?} budget exceeded");
        self.event(
            json!({"event": "charge", "name": name, "kind": format!("{kind:?}"), "index": index}),
        );
    }
    fn png(&mut self, name: &str, pixels: &[u8]) {
        assert_eq!(pixels.len(), W * H * 4, "review PNGs are 960x540 RGBA");
        self.used.pngs += 1;
        assert!(self.used.pngs <= self.limit.pngs, "PNG budget exceeded");
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
    /// Writes report.json and returns whether every contract check passed and
    /// the budget was consumed exactly. Historical-threshold observations and
    /// image metrics are recorded in the body but never decide the outcome.
    fn finish(mut self, mut body: Value, checks: &[(&str, bool)]) -> bool {
        let exact = self.used == self.limit;
        let pass = exact && checks.iter().all(|(_, ok)| *ok);
        let mut named: Map<String, Value> = checks
            .iter()
            .map(|(name, ok)| ((*name).to_owned(), json!(ok)))
            .collect();
        named.insert("budget_exactly_consumed".into(), json!(exact));
        let object = body.as_object_mut().unwrap();
        object.insert("case".into(), json!(self.case));
        object.insert("checks".into(), Value::Object(named));
        object.insert("pngs".into(), json!(self.pngs));
        object.insert("budget_limit".into(), self.limit.json());
        object.insert("budget_used".into(), self.used.json());
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
/// Headless renderer with both startup-only opt-ins, in production order.
fn owner_renderer() -> Renderer {
    let mut renderer = support::checked(Renderer::headless().expect("headless renderer"));
    pollster::block_on(renderer.enable_temporal_upscaling()).expect("enable temporal upscaling");
    pollster::block_on(renderer.enable_motion_owner()).expect("enable motion owner");
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    Full,
    Drs,
    Up,
    Owner,
}
/// One retained scene per variant. UP and OWN make identical calls except the
/// single owner attachment, which resets motion history before upscaling.
fn variant(
    renderer: &Renderer,
    input: &SceneInput,
    camera: &str,
    scale: Option<u8>,
    kind: Variant,
) -> RetainedScene {
    let mut scene = renderer
        .prepare_retained(input, camera)
        .expect("prepare retained scene");
    if let Some(sixteenths) = scale {
        scene
            .set_dynamic_resolution(Some(fixed(sixteenths)))
            .expect("fixed internal scale");
    }
    if matches!(kind, Variant::Up | Variant::Owner) {
        scene
            .enable_motion_history(renderer)
            .expect("motion history");
        if kind == Variant::Owner {
            scene
                .set_motion_owner_attachment(renderer, true)
                .expect("owner attachment");
        }
        scene
            .enable_temporal_upscaling(renderer, REVIEW)
            .expect("output-resolution upscaling owner");
    }
    assert_eq!(
        scene.motion_owner_attachment_enabled(),
        kind == Variant::Owner,
        "owner attachment only on OWN"
    );
    scene
}

// ------------------------------------------------------------------ frames

struct Shot {
    name: String,
    variant: Variant,
    pixels: Vec<u8>,
    frame: Option<UpscaleFrame>,
    internal: Option<[u32; 2]>,
    attached: bool,
}
fn review_target(renderer: &Renderer) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("0073 owner review target"),
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
/// Full readback after the scene (and optional UI) submissions on one queue:
/// one queue submission and one map.
fn read(renderer: &Renderer, texture: &wgpu::Texture) -> Vec<u8> {
    let [width, height] = SIZE;
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("0073 owner review readback"),
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
/// by the physical UI on the same target, then one charged full readback.
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
    report.charge(name, Charge::Scene);
    scene
        .submit(renderer, input, output())
        .expect("retained submit");
    let with_ui = ui.is_some();
    if let Some((layer, ui)) = ui {
        report.charge(name, Charge::Ui);
        let stage = layer.stage(renderer, ui).expect("UI stage");
        layer.submit(renderer, stage, &output()).expect("UI submit");
    }
    report.charge(name, Charge::Readback);
    let pixels = read(renderer, &texture);
    let frame = scene.upscale_frame();
    let internal = scene.resolution_frame().map(|r| r.internal_size());
    let attached = scene.motion_owner_attachment_enabled();
    let motion = frame.as_ref().map(|f| f.motion());
    report.event(json!({
        "event": "shot",
        "name": name,
        "variant": format!("{variant:?}"),
        "ui": with_ui,
        "owner_attached": attached,
        "rgba_sha256": hex(&pixels),
        "phase": motion.and_then(|m| m.jitter_phase()),
        "history": motion.map(|m| m.has_previous()),
        "camera_static": motion.map(|m| m.camera_static()),
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
        attached,
    }
}
/// Expected submitted-owner state of one UP or OWN frame. `epoch` is relative
/// to the first frame of the sequence.
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
/// The plain 8-phase cycle at one internal scale with history after frame 1.
fn cycle(frames: u32, sixteenths: u8) -> Vec<Expect> {
    (0..frames)
        .map(|k| Expect {
            phase: k % 8,
            history: k > 0,
            epoch: 0,
            internal: internal(sixteenths),
        })
        .collect()
}
/// UP never carries the owner attachment and OWN always does.
fn attachment(up: &[&Shot], own: &[&Shot]) -> bool {
    up.iter().all(|s| !s.attached) && own.iter().all(|s| s.attached)
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
                    "owner_attached": s.attached,
                    "rgba_sha256": hex(&s.pixels),
                    "phase": motion.and_then(|m| m.jitter_phase()),
                    "history": motion.map(|m| m.has_previous()),
                    "camera_static": motion.map(|m| m.camera_static()),
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
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .collect::<HashSet<_>>()
        .len()
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

// ------------------------------------------------------------------ owner review metrics

/// Rec. 709 weights on sRGB codes (no linearisation): a stated observation metric.
fn luma(pixels: &[u8], x: usize, y: usize) -> f64 {
    let i = index(x, y);
    0.2126 * f64::from(pixels[i])
        + 0.7152 * f64::from(pixels[i + 1])
        + 0.0722 * f64::from(pixels[i + 2])
}
/// Luminance standard deviation in sRGB codes over `[x, y, w, h]`.
fn contrast(pixels: &[u8], rect: [usize; 4]) -> f64 {
    let [x0, y0, w, h] = rect;
    let n = (w * h).max(1) as f64;
    let (mut sum, mut squares) = (0., 0.);
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let v = luma(pixels, x, y);
            sum += v;
            squares += v * v;
        }
    }
    let mean = sum / n;
    (squares / n - mean * mean).max(0.).sqrt()
}
/// Inner axis-aligned output rectangle of a projected world rectangle on the
/// plane `z`, shrunk by `inset` output pixels (negative grows it).
fn projected_rect(
    camera: DMat4,
    rect: [f32; 4],
    x: [f64; 2],
    y: [f64; 2],
    z: f64,
    inset: f64,
) -> [usize; 4] {
    let p = |px: f64, py: f64| project(camera, rect, [px, py, z]);
    let (tl, tr, bl, br) = (p(x[0], y[1]), p(x[1], y[1]), p(x[0], y[0]), p(x[1], y[0]));
    let left = (tl[0].max(bl[0]) + inset).ceil().max(0.) as usize;
    let top = (tl[1].max(tr[1]) + inset).ceil().max(0.) as usize;
    let right = ((tr[0].min(br[0]) - inset).floor().max(0.) as usize).min(W);
    let bottom = ((bl[1].min(br[1]) - inset).floor().max(0.) as usize).min(H);
    [
        left,
        top,
        right.saturating_sub(left),
        bottom.saturating_sub(top),
    ]
}
/// Fixed chart regions on the sign (world metres on z = 0). They are the
/// 64-texel quadrants' right-hand parts, clear of the sphere and overlays.
const CHART_REGIONS: [(&str, [f64; 2], [f64; 2]); 4] = [
    ("lines_1_texel", [2.95, 3.35], [1.35, 2.25]),
    ("checker_2_texel", [3.45, 4.45], [1.35, 2.25]),
    ("rings", [2.95, 3.35], [0.25, 1.15]),
    ("checker_4_texel", [3.45, 4.45], [0.25, 1.15]),
];
fn chart_rects(camera: DMat4, rect: [f32; 4]) -> Vec<(&'static str, [usize; 4])> {
    CHART_REGIONS
        .iter()
        .map(|&(name, x, y)| (name, projected_rect(camera, rect, x, y, 0., 2.)))
        .collect()
}
fn chart_values(pixels: &[u8], rects: &[(&'static str, [usize; 4])]) -> Vec<f64> {
    rects.iter().map(|(_, r)| contrast(pixels, *r)).collect()
}
fn chart_json(pixels: &[u8], rects: &[(&'static str, [usize; 4])]) -> Value {
    Value::Object(
        rects
            .iter()
            .map(|(name, r)| ((*name).to_owned(), json!(contrast(pixels, *r))))
            .collect(),
    )
}
fn refs(shots: &[Shot]) -> Vec<&Shot> {
    shots.iter().collect()
}

// ------------------------------------------------------------------ historical thresholds

/// Historical C9 threshold values of the linear CPU model, unchanged. Here they
/// are compared with rendered sRGB-code contrast: an observation, not a pass or
/// fail of the historical gate.
const C9_THIRD: f64 = 0.75;
const C9_FOURTH: f64 = 0.90;
const C9_FIRST_LOW: f64 = 0.90;
/// Zero-based indices in SCHEDULE: the last 12/16 frame, the first 4/16 frame,
/// and the third and fourth 16/16 frames after 4/16.
const LAST_12: usize = 12;
const FIRST_4: usize = 13;
const THIRD_16: usize = 18;
const FOURTH_16: usize = 19;
/// A contrast ratio is defined only for a finite numerator over a finite,
/// positive denominator; otherwise it is undefined (null), never a pass.
fn contrast_ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if numerator.is_finite() && denominator.is_finite() && denominator > 0. {
        Some(numerator / denominator)
    } else {
        None
    }
}
/// Threshold comparison of a defined ratio; null when the ratio is undefined.
fn meets(ratio: Option<f64>, threshold: f64) -> Value {
    ratio.map_or(Value::Null, |r| json!(r >= threshold))
}
/// Three-valued all-region summary: false if any region is defined and below
/// the threshold, null if none is below but any is undefined (or there is no
/// region), true only if every region is defined and meets it.
fn all_regions(regions: &[Value], key: &str) -> Value {
    let undefined: Vec<&Value> = regions
        .iter()
        .filter(|v| v[key].as_bool().is_none())
        .map(|v| &v["region"])
        .collect();
    let summary = if regions.iter().any(|v| v[key].as_bool() == Some(false)) {
        json!(false)
    } else if undefined.is_empty() && !regions.is_empty() {
        json!(true)
    } else {
        Value::Null
    };
    json!({"all_regions_meet": summary, "undefined_regions": undefined})
}
/// C9 rendered-output observation. The reference is the mean contrast over
/// frames 17-24 (one 8-phase jitter cycle) of a separate 24-frame fixed-16/16
/// run with identical inputs, viewport and variant: a finite matched-cycle
/// reference, not a demonstrated converged state. Each region is reported.
fn c9_observation(ramp: &[Shot], fixed16: &[Shot], rects: &[(&'static str, [usize; 4])]) -> Value {
    let values = |i: usize| chart_values(&ramp[i].pixels, rects);
    let (last12, first4, third16, fourth16) = (
        values(LAST_12),
        values(FIRST_4),
        values(THIRD_16),
        values(FOURTH_16),
    );
    let (second4, third4) = (values(FIRST_4 + 1), values(FIRST_4 + 2));
    let cycle: Vec<Vec<f64>> = fixed16[16..24]
        .iter()
        .map(|s| chart_values(&s.pixels, rects))
        .collect();
    let regions: Vec<Value> = rects
        .iter()
        .enumerate()
        .map(|(r, (name, rect))| {
            let reference = cycle.iter().map(|v| v[r]).sum::<f64>() / cycle.len() as f64;
            let third = contrast_ratio(third16[r], reference);
            let fourth = contrast_ratio(fourth16[r], reference);
            let low = contrast_ratio(first4[r], last12[r]);
            json!({
                "region": name,
                "rect": rect,
                "matched_cycle_reference": reference,
                "third_16": third16[r],
                "fourth_16": fourth16[r],
                "ratio_third": third,
                "ratio_fourth": fourth,
                "third_meets_0_75": meets(third, C9_THIRD),
                "fourth_meets_0_90": meets(fourth, C9_FOURTH),
                "last_12": last12[r],
                "first_4": first4[r],
                "ratio_first_4": low,
                "first_4_meets_0_90": meets(low, C9_FIRST_LOW),
                "second_and_third_4": [second4[r], third4[r]],
            })
        })
        .collect();
    json!({
        "third_16": all_regions(&regions, "third_meets_0_75"),
        "fourth_16": all_regions(&regions, "fourth_meets_0_90"),
        "first_4": all_regions(&regions, "first_4_meets_0_90"),
        "regions": regions,
    })
}
/// Historical C7 limit value of the linear CPU model, unchanged. Here it is
/// compared with rendered sRGB-code differences: an observation, not a pass or
/// fail of the historical gate.
const C7_LIMIT: f64 = 0.06;
/// Per column: the mean over rows of the mean RGB absolute difference, in
/// normalised sRGB code units (code / 255).
fn column_means(
    a: &[u8],
    b: &[u8],
    columns: std::ops::Range<usize>,
    rows: std::ops::Range<usize>,
) -> Vec<f64> {
    let count = rows.len().max(1) as f64;
    columns
        .map(|x| {
            rows.clone()
                .map(|y| {
                    let i = index(x, y);
                    (0..3)
                        .map(|c| f64::from(a[i + c].abs_diff(b[i + c])))
                        .sum::<f64>()
                        / (3. * 255.)
                })
                .sum::<f64>()
                / count
        })
        .collect()
}
/// C7 rendered-output observation; undefined (null) for an empty column or row
/// range or a nonfinite column value, never an apparent pass.
fn c7_observation(
    moving: &Shot,
    reference: &Shot,
    columns: std::ops::Range<usize>,
    rows: std::ops::Range<usize>,
) -> Value {
    let values = column_means(
        &moving.pixels,
        &reference.pixels,
        columns.clone(),
        rows.clone(),
    );
    let defined = !rows.is_empty() && !values.is_empty() && values.iter().all(|v| v.is_finite());
    let worst = values.iter().copied().fold(0., f64::max);
    json!({
        "columns": [columns.start, columns.end],
        "rows": [rows.start, rows.end],
        "per_column": values,
        "worst_column": if defined { json!(worst) } else { Value::Null },
        "worst_at_most_0_06": if defined { json!(worst <= C7_LIMIT) } else { Value::Null },
    })
}
/// P1 residual against the matched bar-free reference: mean and worst of the
/// per-pixel mean RGB absolute difference (code / 255) over `[x, y, w, h]`.
fn residual(a: &[u8], b: &[u8], rect: [usize; 4]) -> Value {
    let [x0, y0, w, h] = rect;
    let (mut total, mut worst) = (0., 0f64);
    for y in y0..y0 + h {
        for x in x0..x0 + w {
            let i = index(x, y);
            let d = (0..3)
                .map(|c| f64::from(a[i + c].abs_diff(b[i + c])))
                .sum::<f64>()
                / (3. * 255.);
            total += d;
            worst = worst.max(d);
        }
    }
    json!({"rect": rect, "mean": total / (w * h).max(1) as f64, "worst": worst})
}

// ------------------------------------------------------------------ review board

/// C7 and P1 board: the camera 6 m in front of a 12x7 m background on z = 0,
/// actors 4 m (and the P1 bar 5 m) from the eye, a shadowless sun.
const BOARD_EYE: [f64; 3] = [0., 0., 6.];
const ACTOR_Z: f64 = 2.;
const BAR_Z: f64 = 1.;
/// Out of view and static: references keep identical bindings and owners.
const AWAY_ACTOR: [f64; 3] = [40., 0., ACTOR_Z];
const AWAY_BAR: [f64; 3] = [40., 0., BAR_Z];
fn board_camera() -> DMat4 {
    DMat4::from_translation(DVec3::from_array(BOARD_EYE))
}
/// Metres per output pixel on a plane `depth` metres in front of the eye.
fn metres_per_pixel(depth: f64) -> f64 {
    2. * depth * (FOV_DEGREES.to_radians() / 2.).tan() / H as f64
}
/// A camera-facing rectangle with UVs (top-left texel at the top-left corner).
fn plane_mesh(half: [f32; 2]) -> Mesh {
    let corners = [
        Vec3::new(-half[0], -half[1], 0.),
        Vec3::new(half[0], -half[1], 0.),
        Vec3::new(half[0], half[1], 0.),
        Vec3::new(-half[0], half[1], 0.),
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
/// Black-and-white checker (232/28 codes) with square cells.
fn checker(size: i32, cell: i32) -> Vec<u8> {
    let mut pixels: Vec<u8> = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let v: u8 = if (x / cell + y / cell) % 2 == 0 {
                232
            } else {
                28
            };
            pixels.extend([v, v, v, 255]);
        }
    }
    pixels
}
fn lit(color: [f32; 3]) -> Value {
    json!({"pbrMetallicRoughness":{"baseColorFactor":[color[0],color[1],color[2],1],"metallicFactor":0,"roughnessFactor":0.6}})
}
fn lit_texture() -> Value {
    json!({"pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":0.9}})
}
struct Actor {
    stem: &'static str,
    half: [f32; 2],
    material: Value,
    texture: Option<(Vec<u8>, u32)>,
    at: [f64; 3],
}
struct Board {
    f: materials::Fixture,
    camera: String,
    actors: Vec<String>,
}
impl Board {
    /// The fixture's own model becomes the background; actors never cast shadows.
    fn new(textured_background: bool, actors: Vec<Actor>) -> Self {
        let mut f = materials::Fixture::new(json!({}));
        let material = if textured_background {
            lit_texture()
        } else {
            lit([0.5, 0.5, 0.5])
        };
        let (bytes, mut document) = gltf(&plane_mesh([6., 3.5]), "quad.bin", material);
        if textured_background {
            materials::write_png(&f.root.path().join("quad.png"), &checker(256, 4), 256, 256);
            document["images"] = json!([{"uri": "quad.png"}]);
            document["textures"] = json!([{"source":0,"sampler":0}]);
            document["samplers"] =
                json!([{"magFilter":9729,"minFilter":9987,"wrapS":33071,"wrapT":33071}]);
        }
        std::fs::write(f.root.path().join("quad.bin"), &bytes).unwrap();
        std::fs::write(
            f.root.path().join("quad.gltf"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        f.cook();
        f.environment(&[34, 36, 44, 255].repeat(2), 2, 1, TextureUsage::Linear);
        let mut camera = Entity::new("Board camera");
        camera.components.insert(
            "Camera".into(),
            json!({"fov_degrees":FOV_DEGREES,"near":NEAR,"far":FAR}),
        );
        camera.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: BOARD_EYE,
                ..Default::default()
            }),
        );
        let camera = insert(&mut f, camera);
        let mut sun = Entity::new("Board sun");
        sun.components.insert(
            "DirectionalLight".into(),
            json!({"color":[1,0.96,0.9],"intensity":3.2}),
        );
        sun.components.insert(
            "Transform".into(),
            json!(Transform {
                rotation: sun_rotation(25., 10.),
                ..Default::default()
            }),
        );
        insert(&mut f, sun);
        let mut ids = Vec::new();
        for actor in actors {
            let texture = actor
                .texture
                .as_ref()
                .map(|(pixels, size)| (pixels.as_slice(), *size, *size));
            let mesh = plane_mesh(actor.half);
            let asset = add_model(&mut f, actor.stem, &mesh, actor.material, texture);
            ids.push(insert(
                &mut f,
                mesh_entity(
                    actor.stem,
                    &asset,
                    false,
                    Transform {
                        translation: actor.at,
                        ..Default::default()
                    },
                ),
            ));
        }
        f.assets.sync_project(&f.project, f.root.path()).unwrap();
        Self {
            f,
            camera,
            actors: ids,
        }
    }
    fn input(&self) -> SceneInput {
        project_input(&self.f.project, Some(&self.f.assets)).unwrap()
    }
    fn place(&self, input: &mut SceneInput, actor: usize, at: [f64; 3]) {
        let n = node(input, &self.actors[actor]);
        input.nodes[n].world = DMat4::from_translation(DVec3::from_array(at));
    }
}

/// Test-only raw readback of the unchanged R5 reconstruction. No reference
/// classification or timing claim: these bytes are input to a separate replay.
#[test]
#[ignore = "bounded local Metal diagnostic, explicit one-test invocation only"]
fn stationary_raw_history_diagnostic() {
    let out = PathBuf::from(std::env::var("INCANT_RECONSTRUCTION_DIAGNOSTIC_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh diagnostic output directory");
    let stage = Stage::new(false);
    let mut input = stage.input();
    stage.pose(&mut input, 1);
    let renderer = owner_renderer();
    let (_target, view) = review_target(&renderer);
    let mut rows = Vec::new();
    for kind in [Variant::Up, Variant::Owner] {
        let mut scene = variant(&renderer, &input, &stage.camera, Some(HALF), kind);
        for frame in 1..=24 {
            scene
                .submit(
                    &renderer,
                    &input,
                    RetainedTarget {
                        view: &view,
                        format: FORMAT,
                        width: SIZE[0],
                        height: SIZE[1],
                        viewport: None,
                    },
                )
                .unwrap();
            let history = scene
                .upscaling
                .as_ref()
                .unwrap()
                .diagnostic_surfaces()
                .unwrap();
            let current = renderer.frames.diagnostic_color();
            let movement = scene.motion.as_ref().unwrap().readback_view().unwrap();
            let native_depth = scene.motion.as_ref().unwrap().depth_test_view().unwrap();
            let views = [
                history[0].clone(),
                history[1].clone(),
                history[2].clone(),
                current,
                movement,
                native_depth,
            ];
            let labels = [
                "history-color",
                "history-depth",
                "history-evidence",
                "current-color",
                "motion",
                "current-depth",
            ];
            let bpps = [8_u32, 4, 4, 8, 16, 4];
            let formats = [
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureFormat::R32Float,
                wgpu::TextureFormat::Rg16Float,
                wgpu::TextureFormat::Rgba16Float,
                wgpu::TextureFormat::Rgba32Float,
                wgpu::TextureFormat::Depth32Float,
            ];
            let mut planes = Vec::new();
            let mut total = 0_u64;
            for i in 0..6 {
                let t = views[i].texture();
                let expected = if i < 3 { SIZE } else { internal(HALF) };
                assert_eq!([t.width(), t.height()], expected);
                assert_eq!(t.format(), formats[i]);
                // Read the chart and generous surrounding output support. Motion
                // and depth use complete source subresources, including depth's
                // platform copy constraints. Current HDR uses matching support.
                let rect = if i < 3 {
                    [592, 128, 208, 208]
                } else if i == 3 {
                    [294, 62, 108, 108]
                } else {
                    [0, 0, expected[0], expected[1]]
                };
                let stride = (rect[2] * bpps[i]).div_ceil(256) * 256;
                planes.push((rect, stride, total));
                total += u64::from(stride) * u64::from(rect[3]);
            }
            let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("R5 reconstruction diagnostic packed raw planes"),
                size: total,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = renderer.device.create_command_encoder(&Default::default());
            for i in 0..6 {
                let (rect, stride, offset) = planes[i];
                let mut copy = views[i].texture().as_image_copy();
                copy.origin = wgpu::Origin3d {
                    x: rect[0],
                    y: rect[1],
                    z: 0,
                };
                if i == 5 {
                    copy.aspect = wgpu::TextureAspect::DepthOnly;
                }
                encoder.copy_texture_to_buffer(
                    copy,
                    wgpu::TexelCopyBufferInfo {
                        buffer: &buffer,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset,
                            bytes_per_row: Some(stride),
                            rows_per_image: Some(rect[3]),
                        },
                    },
                    wgpu::Extent3d {
                        width: rect[2],
                        height: rect[3],
                        depth_or_array_layers: 1,
                    },
                );
            }
            renderer.queue.submit([encoder.finish()]);
            let (tx, rx) = std::sync::mpsc::channel();
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |value| {
                    tx.send(value).unwrap();
                });
            renderer
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
            let bytes = buffer.slice(..).get_mapped_range();
            let mut files = Vec::new();
            for i in 0..6 {
                let (rect, stride, offset) = planes[i];
                let name = format!("{kind:?}-{frame:02}-{}.bin", labels[i]);
                let data = &bytes[offset as usize..offset as usize + (stride * rect[3]) as usize];
                std::fs::write(out.join(&name), data).unwrap();
                files.push(
                    json!({"name":name,"sha256":hex(data),"rect":rect,"stride":stride,
                    "bytes_per_pixel":bpps[i],"format":format!("{:?}",formats[i])}),
                );
            }
            drop(bytes);
            buffer.unmap();
            let state = scene.upscale_frame().unwrap();
            let jitter = state.motion().jitter_offset().unwrap();
            rows.push(json!({"variant":format!("{kind:?}"),"frame":frame,
                "phase":state.motion().jitter_phase(),"has_previous":state.motion().has_previous(),
                "camera_static":state.motion().camera_static(),"jitter_internal_pixels":jitter,"planes":files}));
        }
    }
    let report = json!({"schema":"incant-reconstruction-raw-diagnostic-v1","baseline_source":"8ae3a3ed9896d534390b2d0e1c2f2d3e2a4df14d",
        "adapter":describe(&renderer),"configuration":policy(),"output_size":SIZE,"source_size":internal(HALF),
        "charts":chart_rects(camera_world(1),FULL_RECT),"submissions":96,"maps":48,"frames":rows,
        "scope":"raw test-only readback of stationary reconstruction candidate; exact source bound externally; no appearance or performance acceptance"});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "RAW_RECONSTRUCTION_DIAGNOSTIC frames=48 submissions=96 maps=48 adapter={:?}",
        renderer.adapter.get_info()
    );
}

#[path = "reconstruction_recurrence_fixture.rs"]
mod recurrence_fixture;
#[path = "reconstruction_reference.rs"]
mod reference_fixture;

#[path = "reconstruction_recurrence_controls.rs"]
mod recurrence_controls;
