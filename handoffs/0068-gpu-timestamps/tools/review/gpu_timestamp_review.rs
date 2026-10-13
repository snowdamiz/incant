//! Packet-local GPU timestamp review helper for handoff 0068; not production
//! code. It is committed only under handoffs/0068-gpu-timestamps/tools/review/
//! and copied into the granted child checkout as
//! crates/incant_render/tests/gpu_timestamp_review.rs by tools/build.py.
//!
//! Appearance cases prepare one SceneInput, then render it through
//! `Renderer::new` (timing disabled) and `Renderer::new_with_gpu_timing` (timing
//! enabled) into the same Rgba8UnormSrgb target size and viewport. Every frame
//! is read back and compared byte for byte; only named frames are written as
//! PNG. Expected timestamp groups come from scene content (model draws, local
//! lights, sprites, text quads), not from the timing code. Durations are kept
//! raw and are not performance measurements. No window, presentation, desktop
//! capture, audio or benchmark. Output goes only to the new directory named by
//! INCANT_0068_REVIEW_OUT.
#![allow(dead_code)] // The shared fixture modules carry helpers these cases do not call.

mod materials;
mod support;

use glam::{DMat4, DQuat, DVec3, Vec3};
use incant_assets::{AssetStore, cook_texture};
use incant_doc::{
    Asset, AssetImportSettings, Camera, CameraProjection, Entity, Project, Scene, SpriteMode,
    SpriteRenderer, SpriteSampling, TextureUsage, Transform,
};
use incant_render::{
    RenderScene, Renderer, SceneInput, Viewport,
    timing::{GPU_PASS_COUNT, GpuFrameTiming, GpuPass, GpuTimingError, GpuTimingStats},
    wgpu,
};
use incant_render_authoring::{RendererAuthoringExt as _, project_input};
use incant_render_data::{SceneLight, TextInput, TextNode};
use incant_text::{AtlasConfig, FontSource, GlyphAtlas, TextEngine, TextStyle, Wrap};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const SIZE: [u32; 2] = [640, 360];
const MAX_SIZE: [u32; 2] = [960, 540];
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansArabic.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
/// The text route's mixed-script control (src/text_tests.rs): a Latin ligature,
/// an Arabic run, CJK fallback and a Hebrew letter no packaged font covers.
const MIXED: &str = "ffi سلام 中文 ש";
/// Dependency rank in the frame graph schedule (src/graph.rs pass_schedule).
const RANK: [u8; GPU_PASS_COUNT] = [0, 0, 0, 1, 2, 3, 4];

// ------------------------------------------------------------------ receipts

/// Append-only events plus one final report, in a directory that must be new.
/// Frame and submission limits are hard: exceeding either panics before work.
struct Report {
    case: &'static str,
    root: PathBuf,
    events: File,
    frame_limit: u32,
    submission_limit: u32,
    frames: u32,
    submissions: u32,
    pngs: Vec<Value>,
}
impl Report {
    fn open(case: &'static str, frame_limit: u32, submission_limit: u32) -> Self {
        let root = PathBuf::from(
            std::env::var_os("INCANT_0068_REVIEW_OUT")
                .expect("INCANT_0068_REVIEW_OUT must name a new output directory"),
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
            submission_limit,
            frames: 0,
            submissions: 0,
            pngs: Vec::new(),
        };
        report.event(json!({
            "event": "open",
            "case": case,
            "frame_limit": frame_limit,
            "submission_limit": submission_limit,
        }));
        report
    }
    fn event(&mut self, value: Value) {
        writeln!(self.events, "{value}").unwrap();
        self.events.flush().unwrap();
    }
    fn count_submission(&mut self) {
        self.submissions += 1;
        assert!(
            self.submissions <= self.submission_limit,
            "numeric submission budget exceeded"
        );
    }
    fn png(&mut self, name: &str, pixels: &[u8]) {
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
    /// Writes report.json and returns whether every acceptance check passed.
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
        object.insert("frames_rendered".into(), json!(self.frames));
        object.insert("timed_submissions".into(), json!(self.submissions));
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

// ------------------------------------------------------------------ renderers

fn open_renderer(
    instance: &wgpu::Instance,
    timing: bool,
) -> Result<Renderer, Box<dyn std::error::Error + Send + Sync>> {
    if timing {
        pollster::block_on(Renderer::new_with_gpu_timing(instance, None))
    } else {
        pollster::block_on(Renderer::new(instance, None))
    }
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
        "device_features": format!("{:?}", renderer.device.features()),
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
fn prepare(renderer: &Renderer, input: &SceneInput, camera: &str) -> RenderScene {
    renderer
        .prepare(input)
        .expect("prepare scene")
        .with_camera(camera)
        .expect("select camera")
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
/// One complete offscreen frame: draw_scene, readback copy, blocking map.
fn frame(
    report: &mut Report,
    renderer: &Renderer,
    scene: &RenderScene,
    viewport: Option<Viewport>,
    name: &str,
) -> Vec<u8> {
    let [width, height] = SIZE;
    assert!(width <= MAX_SIZE[0] && height <= MAX_SIZE[1]);
    report.frames += 1;
    assert!(
        report.frames <= report.frame_limit,
        "appearance frame budget exceeded"
    );
    let timed = renderer.gpu_timing_stats().is_ok_and(|s| s.enabled);
    if timed {
        report.count_submission();
    }
    report.event(json!({"event": "frame", "name": name, "timed": timed, "index": report.frames}));
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("0068 review target"),
        size: wgpu::Extent3d {
            width,
            height,
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
    let commands = renderer
        .draw_scene(scene, &view, FORMAT, width, height, viewport)
        .expect("draw_scene");
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("0068 review readback"),
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
    renderer.queue.submit([commands, encoder.finish()]);
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
    let polls = settle(renderer);
    report.event(json!({
        "event": "frame-read",
        "name": name,
        "rgba_sha256": hex(&pixels),
        "settle_polls": polls,
        "stats": stats_value(renderer),
    }));
    pixels
}

// ------------------------------------------------------------------ timing

fn labels(groups: [bool; GPU_PASS_COUNT]) -> Value {
    let map: Map<String, Value> = GpuPass::ALL
        .iter()
        .zip(groups)
        .map(|(pass, present)| (format!("{pass:?}"), json!(present)))
        .collect();
    Value::Object(map)
}
fn local_lights(input: &SceneInput) -> usize {
    input
        .nodes
        .iter()
        .filter(|node| matches!(node.light, Some(SceneLight::Point(_) | SceneLight::Spot(_))))
        .count()
}
/// Groups the frame graph encodes for this content (src/graph.rs systems).
fn expected_groups(scene: &RenderScene, input: &SceneInput) -> [bool; GPU_PASS_COUNT] {
    let stats = scene.stats();
    let models = stats.model_draw_calls > 0;
    let lights = local_lights(input) > 0;
    GpuPass::ALL.map(|pass| match pass {
        GpuPass::InitializeScene | GpuPass::Display => true,
        GpuPass::AssignLights => models && lights,
        GpuPass::Shadows | GpuPass::Models => models,
        GpuPass::Sprites => stats.sprite_entities + stats.tilemap_entities > 0,
        GpuPass::Text => stats.text_glyph_quads > 0,
    })
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
/// Contract checks for one sample, plus observations that never decide outcome.
fn sample_checks(sample: &GpuFrameTiming, expected: [bool; GPU_PASS_COUNT]) -> (bool, Value) {
    let present = sample.passes.map(|span| span.is_some());
    let labels_match = present == expected;
    let spans_valid = sample
        .passes
        .iter()
        .flatten()
        .chain([&sample.frame])
        .all(|s| s.end_ticks >= s.start_ticks && s.nanoseconds.is_finite() && s.nanoseconds >= 0.);
    let period_valid = sample.timestamp_period_ns.is_finite() && sample.timestamp_period_ns > 0.;
    let within_frame = sample.passes.iter().flatten().all(|s| {
        s.start_ticks >= sample.frame.start_ticks && s.end_ticks <= sample.frame.end_ticks
    });
    let mut order_violations = Vec::new();
    for (a, first) in sample.passes.iter().enumerate() {
        for (b, second) in sample.passes.iter().enumerate() {
            if let (Some(first), Some(second)) = (first, second)
                && RANK[a] < RANK[b]
                && first.end_ticks > second.start_ticks
            {
                order_violations.push(format!("{:?} > {:?}", GpuPass::ALL[a], GpuPass::ALL[b]));
            }
        }
    }
    let zero: Vec<_> = GpuPass::ALL
        .iter()
        .zip(sample.passes)
        .filter(|(_, span)| span.is_some_and(|s| s.start_ticks == s.end_ticks))
        .map(|(pass, _)| format!("{pass:?}"))
        .collect();
    (
        labels_match && spans_valid && period_valid,
        json!({
            "sequence": sample.sequence,
            "labels_match_expected": labels_match,
            "present": labels(present),
            "spans_ordered_finite_nonnegative": spans_valid,
            "period_finite_positive": period_valid,
            "observation_groups_within_frame": within_frame,
            "observation_dependency_order_violations": order_violations,
            "observation_zero_tick_groups": zero,
        }),
    )
}

// ------------------------------------------------------------------ pixels

fn compare(a: &[u8], b: &[u8]) -> Value {
    let width = SIZE[0] as usize;
    let mut differing = 0usize;
    let mut max_delta = 0u8;
    let mut first = None;
    for (index, (x, y)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        if x != y {
            differing += 1;
            let delta = x
                .iter()
                .zip(y)
                .map(|(p, q)| p.abs_diff(*q))
                .max()
                .unwrap_or(0);
            max_delta = max_delta.max(delta);
            first.get_or_insert([index % width, index / width]);
        }
    }
    json!({
        "equal": a == b,
        "differing_pixels": differing,
        "max_channel_delta": max_delta,
        "first_difference": first,
    })
}
fn content(pixels: &[u8]) -> Value {
    let distinct: HashSet<&[u8]> = pixels.chunks_exact(4).collect();
    let [width, height] = SIZE.map(|v| v as usize);
    let at = |x: usize, y: usize| pixels[(y * width + x) * 4..(y * width + x) * 4 + 4].to_vec();
    json!({
        "rgba_sha256": hex(pixels),
        "distinct_colors": distinct.len(),
        "top_left": at(0, 0),
        "center": at(width / 2, height / 2),
        "bottom_right": at(width - 1, height - 1),
    })
}

// ------------------------------------------------------------------ fixtures

/// Shadow look-dev geometry, adapted from tests/shadows/lookdev.rs where these
/// builders are private to the model_gpu suite.
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
fn write_lookdev(f: &mut materials::Fixture) {
    let mesh = lookdev_mesh();
    let count = mesh.positions.len() / 3;
    let (min, max) =
        mesh.positions
            .chunks(3)
            .fold(([f32::MAX; 3], [f32::MIN; 3]), |(mut lo, mut hi), p| {
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
                (lo, hi)
            });
    let bytes: Vec<u8> = mesh
        .positions
        .iter()
        .chain(&mesh.normals)
        .flat_map(|v| v.to_le_bytes())
        .collect();
    let half = count * 12;
    std::fs::write(f.root.path().join("quad.bin"), &bytes).unwrap();
    let document = json!({
        "asset":{"version":"2.0"}, "buffers":[{"uri":"quad.bin","byteLength":bytes.len()}],
        "bufferViews":[{"buffer":0,"byteLength":half},{"buffer":0,"byteOffset":half,"byteLength":half}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":count,"type":"VEC3","min":min,"max":max},
            {"bufferView":1,"componentType":5126,"count":count,"type":"VEC3"}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1},"material":0}]}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.62,0.62,0.6,1],"metallicFactor":0,"roughnessFactor":0.85}}],
        "nodes":[{"mesh":0}], "scenes":[{"nodes":[0]}],"scene":0
    });
    std::fs::write(
        f.root.path().join("quad.gltf"),
        serde_json::to_vec(&document).unwrap(),
    )
    .unwrap();
    f.cook();
}
fn sun_rotation(elevation_degrees: f64, azimuth_degrees: f64) -> [f64; 4] {
    (DQuat::from_rotation_y(azimuth_degrees.to_radians())
        * DQuat::from_rotation_x(-elevation_degrees.to_radians()))
    .to_array()
}
/// Look-dev geometry, environment fill, a shadowing sun and two point lights.
struct Lit {
    f: materials::Fixture,
    camera: String,
}
impl Lit {
    fn new() -> Self {
        let mut f = materials::Fixture::new(json!({}));
        write_lookdev(&mut f);
        f.environment(&[34, 36, 44, 255].repeat(2), 2, 1, TextureUsage::Linear);
        let entities = &mut f.project.scenes.values_mut().next().unwrap().entities;
        let mut camera = Entity::new("Review camera");
        camera.components.insert(
            "Camera".into(),
            json!({"fov_degrees":50,"near":0.1,"far":120}),
        );
        camera.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [0.4, 4.2, 8.5],
                rotation: DQuat::from_rotation_x(-0.42).to_array(),
                ..Default::default()
            }),
        );
        let camera_id = camera.id.clone();
        entities.insert(camera_id.clone(), camera);
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
        entities.insert(sun.id.clone(), sun);
        for (name, color, at) in [
            ("Warm local light", [1.0, 0.62, 0.32], [-1.3, 1.1, 1.7]),
            ("Cool local light", [0.35, 0.6, 1.0], [2.2, 1.0, 0.4]),
        ] {
            let mut light = Entity::new(name);
            light.components.insert(
                "PointLight".into(),
                json!({"color":color,"intensity":6,"range":5}),
            );
            light.components.insert(
                "Transform".into(),
                json!(Transform {
                    translation: at,
                    ..Default::default()
                }),
            );
            entities.insert(light.id.clone(), light);
        }
        Self {
            f,
            camera: camera_id,
        }
    }
    fn input(&self) -> SceneInput {
        project_input(&self.f.project, Some(&self.f.assets)).unwrap()
    }
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
fn text_input() -> TextInput {
    let mut engine = TextEngine::new(
        [LATIN, ARABIC, CJK]
            .into_iter()
            .zip(1u128..)
            .map(|(bytes, id)| FontSource {
                id: id.into(),
                bytes: bytes.to_vec(),
            })
            .collect(),
    )
    .unwrap();
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
            id: incant_types::StableId([id; 16]),
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
/// Pixel-perfect camera, three nearest-sampled sprites and world-space text.
struct SpritesText {
    root: tempfile::TempDir,
    project: Project,
    assets: AssetStore,
    camera: String,
}
impl SpritesText {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut project = Project::empty("0068 sprites and text review");
        let mut scene = Scene::new("Main");
        let mut camera = Entity::new("Pixel camera");
        camera.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [0., 0., 8.],
                ..Default::default()
            }),
        );
        camera.components.insert(
            "Camera".into(),
            json!(Camera {
                fov_degrees: 60.,
                near: 0.1,
                far: 100.,
                projection: CameraProjection::PixelPerfect {
                    pixels_per_unit: 100.,
                    reference_resolution: [320, 180]
                }
            }),
        );
        let camera_id = camera.id.clone();
        scene.entities.insert(camera_id.clone(), camera);
        let texture = incant_doc::new_id();
        for (index, (x, y, tint, flip_x, order)) in [
            (-1.0, -0.4, [1.0, 0.55, 0.45, 1.0], false, 0),
            (-0.15, -0.4, [0.55, 1.0, 0.6, 1.0], false, 0),
            (0.3, -0.3, [0.5, 0.7, 1.0, 1.0], true, 1),
        ]
        .into_iter()
        .enumerate()
        {
            let mut sprite = Entity::new(format!("Sprite {index}"));
            sprite.components.insert(
                "Transform".into(),
                json!(Transform {
                    translation: [x, y, 0.],
                    ..Default::default()
                }),
            );
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
            scene.entities.insert(sprite.id.clone(), sprite);
        }
        project.scenes.insert(scene.id.clone(), scene);
        materials::write_png(&root.path().join("badge.png"), &badge(), 16, 16);
        let cooked = cook_texture(
            root.path(),
            Path::new("badge.png"),
            &root.path().join(".incant/cache/textures"),
            TextureUsage::Color,
        )
        .unwrap();
        project.assets.insert(
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
        let mut assets = AssetStore::default();
        assets.sync_project(&project, root.path()).unwrap();
        Self {
            root,
            project,
            assets,
            camera: camera_id,
        }
    }
    fn input(&self) -> SceneInput {
        let mut input = project_input(&self.project, Some(&self.assets)).unwrap();
        input.text = Some(text_input());
        input
    }
}

// ------------------------------------------------------------------ cases

fn timing_stats(pending: usize, ready: usize, skipped: u64) -> GpuTimingStats {
    GpuTimingStats {
        enabled: true,
        pending,
        ready,
        skipped,
        abandoned: 0,
        retired: 0,
    }
}
fn scene_json(scene: &RenderScene, input: &SceneInput, viewport: Option<Viewport>) -> Value {
    json!({
        "stats": scene.stats(),
        "local_lights": local_lights(input),
        "expected_groups": labels(expected_groups(scene, input)),
        "format": "Rgba8UnormSrgb",
        "size": SIZE,
        "viewport": viewport.map(|v| json!({
            "rect": v.rect,
            "corner_radii": v.corner_radii,
            "canvas_srgb": v.canvas_srgb,
        })),
    })
}
/// One timing-disabled and one timing-enabled frame of identical inputs.
fn matched_pair(
    case: &'static str,
    input: &SceneInput,
    camera: &str,
    viewport: Option<Viewport>,
    names: [&str; 2],
) {
    let mut report = Report::open(case, 2, 1);
    let instance = wgpu::Instance::default();
    let off = open_renderer(&instance, false).expect("default renderer");
    let scene = prepare(&off, input, camera);
    let expected = expected_groups(&scene, input);
    let scene_off = scene_json(&scene, input, viewport);
    let pixels_off = frame(&mut report, &off, &scene, viewport, "off-1");
    let renderer_off = describe(&off);
    drop(scene);
    drop(off);
    let on = open_renderer(&instance, true).expect("timing renderer");
    let scene = prepare(&on, input, camera);
    let expected_on = expected_groups(&scene, input);
    let pixels_on = frame(&mut report, &on, &scene, viewport, "on-1");
    let after_frame = on.gpu_timing_stats().unwrap();
    let (sample, first) = take(&on);
    let (extra, second) = take(&on);
    let final_stats = on.gpu_timing_stats().unwrap();
    let renderer_on = describe(&on);
    drop(scene);
    drop(on);
    report.png(names[0], &pixels_off);
    report.png(names[1], &pixels_on);
    let (sample_ok, sample_report) = sample
        .as_ref()
        .map(|s| sample_checks(s, expected))
        .unwrap_or((false, Value::Null));
    let pixels = compare(&pixels_off, &pixels_on);
    let checks = [
        ("pixels_identical_timing_off_vs_on", pixels_off == pixels_on),
        (
            "expected_groups_same_on_both_devices",
            expected == expected_on,
        ),
        (
            "one_sample_with_sequence_1",
            sample.is_some_and(|s| s.sequence == 1),
        ),
        ("sample_contract", sample_ok),
        (
            "no_second_sample",
            extra.is_none() && second["outcome"] == "none",
        ),
        ("stats_after_frame", after_frame == timing_stats(0, 1, 0)),
        ("stats_after_take", final_stats == timing_stats(0, 0, 0)),
    ];
    let body = json!({
        "scene": scene_off,
        "renderers": {"off": renderer_off, "on": renderer_on},
        "frames": {"off-1": content(&pixels_off), "on-1": content(&pixels_on)},
        "comparisons": {"on-1 vs off-1": pixels},
        "timing": {
            "stats_after_frame": after_frame,
            "takes": [first, second],
            "stats_final": final_stats,
            "sample_checks": sample_report,
        },
    });
    assert!(
        report.finish(body, &checks),
        "{case} failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0068 packet review; run only by the guarded packet runner"]
fn probe() {
    let mut report = Report::open("probe", 0, 0);
    let instance = wgpu::Instance::default();
    let off = open_renderer(&instance, false).expect("default renderer");
    let off_stats = off.gpu_timing_stats().unwrap();
    let (off_sample, off_take) = take(&off);
    let off_features = off.device.features();
    let disabled = json!({"renderer": describe(&off), "take": off_take});
    drop(off);
    let (enabled, on_checks) = match open_renderer(&instance, true) {
        Ok(on) => {
            let stats = on.gpu_timing_stats().unwrap();
            let (sample, on_take) = take(&on);
            let features = on.device.features();
            let period = on.queue.get_timestamp_period();
            let value = json!({"renderer": describe(&on), "take": on_take});
            (
                value,
                [
                    ("timing_constructor_succeeded", true),
                    (
                        "timing_device_has_encoder_timestamps",
                        features.contains(
                            wgpu::Features::TIMESTAMP_QUERY
                                | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
                        ),
                    ),
                    ("timing_stats_empty", stats == timing_stats(0, 0, 0)),
                    ("timing_take_none_before_frames", sample.is_none()),
                    ("period_finite_positive", period.is_finite() && period > 0.),
                ],
            )
        }
        Err(error) => (
            json!({"refused": error.to_string()}),
            [
                ("timing_constructor_succeeded", false),
                ("timing_device_has_encoder_timestamps", false),
                ("timing_stats_empty", false),
                ("timing_take_none_before_frames", false),
                ("period_finite_positive", false),
            ],
        ),
    };
    let mut checks = vec![
        (
            "default_device_requests_no_timestamp_feature",
            !off_features.intersects(
                wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
            ),
        ),
        (
            "default_stats_disabled",
            off_stats == GpuTimingStats::default(),
        ),
        ("default_take_none", off_sample.is_none()),
    ];
    checks.extend(on_checks);
    let body = json!({"disabled": disabled, "enabled": enabled});
    assert!(
        report.finish(body, &checks),
        "probe failed; see report.json"
    );
}

#[test]
#[ignore = "handoff 0068 packet review; run only by the guarded packet runner"]
fn lit() {
    let mut report = Report::open("lit", 8, 5);
    let lit = Lit::new();
    let input = lit.input();
    let instance = wgpu::Instance::default();
    // Timing disabled: two frames on one device, one on a second device.
    let off = open_renderer(&instance, false).expect("default renderer");
    let scene = prepare(&off, &input, &lit.camera);
    let expected = expected_groups(&scene, &input);
    let scene_off = scene_json(&scene, &input, None);
    let off_a1 = frame(&mut report, &off, &scene, None, "off-a-1");
    let off_a2 = frame(&mut report, &off, &scene, None, "off-a-2");
    let renderer_off_a = describe(&off);
    drop(scene);
    drop(off);
    let off = open_renderer(&instance, false).expect("second default renderer");
    let scene = prepare(&off, &input, &lit.camera);
    let off_b1 = frame(&mut report, &off, &scene, None, "off-b-1");
    let renderer_off_b = describe(&off);
    drop(scene);
    drop(off);
    // Timing enabled: three measured frames leave three unread results in the
    // fixed slots; the fourth must still render with instrumentation skipped.
    // Draining frees the slots; a fifth frame reuses one.
    let on = open_renderer(&instance, true).expect("timing renderer");
    let scene = prepare(&on, &input, &lit.camera);
    let expected_on = expected_groups(&scene, &input);
    let mut on_frames = Vec::new();
    let mut stats_after = Vec::new();
    for index in 1..=4 {
        on_frames.push(frame(
            &mut report,
            &on,
            &scene,
            None,
            &format!("on-{index}"),
        ));
        stats_after.push(on.gpu_timing_stats().unwrap());
    }
    let drained: Vec<_> = (0..4).map(|_| take(&on)).collect();
    let after_drain = on.gpu_timing_stats().unwrap();
    on_frames.push(frame(&mut report, &on, &scene, None, "on-5"));
    let after_reuse = on.gpu_timing_stats().unwrap();
    let reused = [take(&on), take(&on)];
    let final_stats = on.gpu_timing_stats().unwrap();
    let renderer_on = describe(&on);
    drop(scene);
    drop(on);
    report.png("lit-off-1.png", &off_a1);
    report.png("lit-on-1.png", &on_frames[0]);
    report.png("lit-on-4-skipped.png", &on_frames[3]);

    let drained_sequences: Vec<_> = drained.iter().map(|(s, _)| s.map(|s| s.sequence)).collect();
    let reused_sequences: Vec<_> = reused.iter().map(|(s, _)| s.map(|s| s.sequence)).collect();
    let mut sample_reports = Vec::new();
    let mut samples_ok = true;
    for sample in drained
        .iter()
        .chain(&reused)
        .filter_map(|(s, _)| s.as_ref())
    {
        let (ok, value) = sample_checks(sample, expected);
        samples_ok &= ok;
        sample_reports.push(value);
    }
    let mut frames = Map::new();
    let mut comparisons = Map::new();
    let mut identical = true;
    for (name, pixels) in [
        ("off-a-1", &off_a1),
        ("off-a-2", &off_a2),
        ("off-b-1", &off_b1),
    ]
    .into_iter()
    .chain(
        ["on-1", "on-2", "on-3", "on-4", "on-5"]
            .into_iter()
            .zip(on_frames.iter()),
    ) {
        frames.insert(name.into(), content(pixels));
        if name != "off-a-1" {
            identical &= *pixels == off_a1;
            comparisons.insert(format!("{name} vs off-a-1"), compare(&off_a1, pixels));
        }
    }
    let expected_stats = [
        timing_stats(0, 1, 0),
        timing_stats(0, 2, 0),
        timing_stats(0, 3, 0),
        timing_stats(0, 3, 1),
    ];
    let checks = [
        ("pixels_identical_all_eight_frames", identical),
        ("same_device_determinism_off_a2", off_a2 == off_a1),
        ("cross_device_control_off_b1", off_b1 == off_a1),
        (
            "timing_on_matches_off_1_to_3",
            on_frames[..3].iter().all(|p| *p == off_a1),
        ),
        ("skipped_frame_4_renders_unchanged", on_frames[3] == off_a1),
        ("reused_slot_frame_5_unchanged", on_frames[4] == off_a1),
        (
            "expected_groups_same_on_both_devices",
            expected == expected_on,
        ),
        ("stats_fill_then_skip", stats_after == expected_stats),
        (
            "drain_yields_sequences_1_2_3_then_none",
            drained_sequences == [Some(1), Some(2), Some(3), None],
        ),
        ("stats_after_drain", after_drain == timing_stats(0, 0, 1)),
        (
            "stats_after_reuse_frame",
            after_reuse == timing_stats(0, 1, 1),
        ),
        (
            "reuse_yields_sequence_5_then_none",
            reused_sequences == [Some(5), None],
        ),
        ("stats_final", final_stats == timing_stats(0, 0, 1)),
        (
            "every_sample_meets_contract",
            samples_ok && sample_reports.len() == 4,
        ),
    ];
    let body = json!({
        "scene": scene_off,
        "renderers": {"off-a": renderer_off_a, "off-b": renderer_off_b, "on": renderer_on},
        "frames": frames,
        "comparisons": comparisons,
        "timing": {
            "stats_after_on_frames_1_to_4": stats_after,
            "drain": drained.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
            "stats_after_drain": after_drain,
            "stats_after_on_frame_5": after_reuse,
            "reuse": reused.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
            "stats_final": final_stats,
            "sample_checks": sample_reports,
        },
    });
    assert!(report.finish(body, &checks), "lit failed; see report.json");
}

#[test]
#[ignore = "handoff 0068 packet review; run only by the guarded packet runner"]
fn sprites_text() {
    let fixture = SpritesText::new();
    let input = fixture.input();
    matched_pair(
        "sprites_text",
        &input,
        &fixture.camera,
        None,
        ["sprites-text-off.png", "sprites-text-on.png"],
    );
}

#[test]
#[ignore = "handoff 0068 packet review; run only by the guarded packet runner"]
fn viewport() {
    let lit = Lit::new();
    let input = lit.input();
    let viewport = Viewport {
        rect: [52.5, 30.25, 512., 288.],
        corner_radii: [12.; 4],
        canvas_srgb: [64, 80, 96],
    };
    matched_pair(
        "viewport",
        &input,
        &lit.camera,
        Some(viewport),
        ["viewport-off.png", "viewport-on.png"],
    );
}

/// Requires Astra's review before execution (brief: callback-after-owner-drop).
/// Observes only that the retained device completes the submission without a
/// panic or hang after the owner is gone; the orphaned result is unobservable.
/// This does not prove callback-after-drop ordering: queue.submit may deliver
/// the callback before drop(renderer). No callback completion flag is exposed.
#[test]
#[ignore = "handoff 0068 packet review; run only by the guarded packet runner"]
fn owner_drop() {
    let mut report = Report::open("owner_drop", 0, 1);
    let instance = wgpu::Instance::default();
    let renderer = open_renderer(&instance, true).expect("timing renderer");
    let directory = tempfile::tempdir().unwrap();
    let (project, assets, _) = support::fixture(directory.path());
    let scene = renderer.prepare_scene(&project, &assets).unwrap();
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("0068 owner-drop target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let commands = renderer
        .draw_scene(&scene, &view, FORMAT, 64, 64, None)
        .unwrap();
    let before = renderer.gpu_timing_stats().unwrap();
    let device = renderer.device.clone();
    let queue = renderer.queue.clone();
    report.count_submission();
    let submission = queue.submit([commands]);
    drop(scene);
    drop(view);
    drop(texture);
    drop(renderer);
    report.event(json!({"event": "owner-dropped"}));
    let waited = match device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(Duration::from_secs(10)),
    }) {
        Ok(status) => json!({"ok": format!("{status:?}")}),
        Err(error) => json!({"error": error.to_string()}),
    };
    let poll_ok = waited.get("ok").is_some();
    drop(queue);
    drop(device);
    let fresh = open_renderer(&instance, true).expect("fresh timing renderer");
    let fresh_stats = fresh.gpu_timing_stats().unwrap();
    let (fresh_sample, fresh_take) = take(&fresh);
    drop(fresh);
    let checks = [
        ("one_pending_before_submit", before == timing_stats(1, 0, 0)),
        ("retained_device_wait_ok_after_owner_drop", poll_ok),
        (
            "fresh_owner_starts_empty",
            fresh_stats == timing_stats(0, 0, 0),
        ),
        ("fresh_owner_take_none", fresh_sample.is_none()),
    ];
    let body = json!({
        "stats_before_submit": before,
        "retained_device_wait": waited,
        "fresh_stats": fresh_stats,
        "fresh_take": fresh_take,
    });
    assert!(
        report.finish(body, &checks),
        "owner_drop failed; see report.json"
    );
}
