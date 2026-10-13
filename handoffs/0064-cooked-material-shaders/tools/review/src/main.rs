//! Packet-local offscreen review helper for handoff 0064; not production code.
//!
//! Loads cooked native scenes exactly as a shipped world does (NativeWorld from
//! cooked bytes, NativeRenderInput from model/texture caches) and submits frames
//! through the retained native renderer into an offscreen Rgba8UnormSrgb target.
//! Every frame is read back and hashed (whole frame and per row). Only frames
//! the plan names are written as PNG, within a hard budget of 24 at <=960x540.
//! Optional in-process controls re-render the same input through the
//! non-retained path, with clustered and all-light selection, and compare.
//! Despawns exercise topology rebuilds (replacement retained scenes) with a
//! changed light count. No project, editor, window, script, audio or presentation.
use incant_render::{LocalLightSelection, Renderer, RetainedScene, RetainedTarget, wgpu};
use incant_render_data::{SceneInput, SceneLight};
use incant_render_native::{NativeRenderInput, UpdateKind, VisualCaches};
use incant_runtime::{FrameCommands, NativeWorld, StableId, StructuralCommand};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

type Error = Box<dyn std::error::Error + Send + Sync>;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const MAX_PNGS: usize = 24;
const MAX_WIDTH: u32 = 960;
const MAX_HEIGHT: u32 = 540;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    png_budget: usize,
    steps: Vec<Step>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    label: String,
    scene: PathBuf,
    models: PathBuf,
    textures: PathBuf,
    camera: String,
    frames: Vec<Frame>,
    #[serde(default)]
    despawn: Vec<Despawn>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    tick: u64,
    width: u32,
    height: u32,
    #[serde(default)]
    png: Option<String>,
    #[serde(default)]
    controls: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Despawn {
    tick: u64,
    entities: Vec<String>,
}

impl Plan {
    fn validate(&self) -> Result<(), Error> {
        let pngs: Vec<_> = self
            .steps
            .iter()
            .flat_map(|s| s.frames.iter().filter_map(|f| f.png.as_ref()))
            .collect();
        if self.png_budget > MAX_PNGS || pngs.len() > self.png_budget {
            return Err("PNG count exceeds the plan budget or the packet maximum of 24".into());
        }
        if pngs.iter().collect::<BTreeSet<_>>().len() != pngs.len()
            || pngs.iter().any(|n| {
                !n.ends_with(".png")
                    || n.len() > 64
                    || !n
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
            })
        {
            return Err("PNG names must be unique, short, plain ASCII .png file names".into());
        }
        if self.steps.is_empty() || self.steps.len() > 24 {
            return Err("plan needs 1..=24 steps".into());
        }
        for step in &self.steps {
            id(&step.camera)?;
            if step.frames.is_empty()
                || step.frames.len() > 48
                || step.frames.windows(2).any(|w| w[0].tick > w[1].tick)
                || step.frames.last().is_some_and(|f| f.tick > 3600)
                || step.frames.iter().any(|f| {
                    !(16..=MAX_WIDTH).contains(&f.width) || !(16..=MAX_HEIGHT).contains(&f.height)
                })
            {
                return Err(format!(
                    "step {}: invalid frames (order, count, size <=960x540)",
                    step.label
                )
                .into());
            }
            for d in &step.despawn {
                if d.tick == 0 || d.tick > step.frames.last().unwrap().tick || d.entities.is_empty()
                {
                    return Err(format!(
                        "step {}: despawn ticks must be within the frames",
                        step.label
                    )
                    .into());
                }
                for e in &d.entities {
                    id(e)?;
                }
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), Error> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: material_review <plan.json> <new-output-directory>".into());
    }
    let plan: Plan = serde_json::from_slice(&read(Path::new(&args[0]), 1024 * 1024)?)?;
    plan.validate()?;
    let output = Path::new(&args[1]);
    std::fs::create_dir(output)?;
    let started = Instant::now();
    let renderer = Renderer::headless()?;
    let info = renderer.adapter.get_info();
    let adapter = json!({"name": info.name, "backend": format!("{:?}", info.backend),
        "device_type": format!("{:?}", info.device_type), "driver": info.driver,
        "driver_info": info.driver_info, "vendor": info.vendor, "device": info.device});
    let mut records = Vec::new();
    let mut pngs_written = 0usize;
    for step in &plan.steps {
        run_step(&renderer, step, output, &mut pngs_written, &mut records)?;
    }
    renderer.check_device()?;
    let report = json!({"adapter": adapter, "frames": records, "pngs_written": pngs_written,
        "png_budget": plan.png_budget, "seconds": started.elapsed().as_secs_f64(),
        "scope": "offscreen retained native frames from cooked scenes; no window, scripts, audio or presentation"});
    let text = serde_json::to_string_pretty(&report)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("report.json"))?
        .write_all(text.as_bytes())?;
    println!(
        "{}",
        serde_json::to_string(
            &json!({"adapter": report["adapter"], "frames": records.len(),
            "pngs_written": pngs_written})
        )?
    );
    Ok(())
}

fn run_step(
    renderer: &Renderer,
    step: &Step,
    output: &Path,
    pngs_written: &mut usize,
    records: &mut Vec<Value>,
) -> Result<(), Error> {
    let mut world = NativeWorld::from_bytes(&read(&step.scene, 128 * 1024 * 1024)?)?;
    if world.audio_nodes().len() != 0 {
        return Err("material_review has no audio host".into());
    }
    let mut input = NativeRenderInput::load(
        &world.spatial_view()?,
        VisualCaches {
            models: &step.models,
            textures: &step.textures,
        },
        id(&step.camera)?,
    )?;
    let mut retained = renderer.prepare_retained(input.input(), input.camera_label())?;
    let mut prepared = 1u32;
    for frame in &step.frames {
        while world.tick() < frame.tick {
            world.step()?;
            let tick = world.tick();
            let mut commands = FrameCommands::default();
            for d in step.despawn.iter().filter(|d| d.tick == tick) {
                for e in &d.entities {
                    commands.push(StructuralCommand::Despawn(id(e)?))?;
                }
            }
            world.apply(&mut commands)?;
        }
        let target = Target::new(renderer, frame.width, frame.height);
        let kind = submit(renderer, &world, &mut input, &mut retained, &target)?;
        if kind == UpdateKind::Rebuilt {
            prepared += 1;
        }
        let rgba = target.read(renderer)?;
        let mut record = describe(&rgba, frame.width);
        let scene = input.input();
        let stats = retained.stats();
        let more = json!({"step": step.label, "tick": frame.tick, "width": frame.width,
            "height": frame.height, "update": format!("{kind:?}"), "retained_scenes_prepared": prepared,
            "entities": world.entity_count(), "tick_rate": world.tick_rate(),
            "binding_generation": input.binding_generation(),
            "topology_generation": input.topology_generation(),
            "lights": lights(scene, frame.width, frame.height),
            "stats": {"model_entities": stats.model_entities, "primitive_instances": stats.primitive_instances,
                "model_triangles": stats.model_triangles, "model_draw_calls": stats.model_draw_calls,
                "diagnostic_entities": stats.diagnostic_entities, "sprite_entities": stats.sprite_entities}});
        merge(&mut record, more);
        if let Some(name) = &frame.png {
            if *pngs_written >= MAX_PNGS {
                return Err("PNG budget exhausted".into());
            }
            let path = output.join(name);
            write_png(&path, frame.width, frame.height, &rgba)?;
            *pngs_written += 1;
            record["png"] =
                json!({"name": name, "sha256": hex(&Sha256::digest(std::fs::read(&path)?))});
        }
        if frame.controls {
            let label = input.camera_label().to_owned();
            let mut controls = serde_json::Map::new();
            // The same accepted input resubmitted to the retained scene must repeat exactly.
            let repeat = Target::new(renderer, frame.width, frame.height);
            if submit(renderer, &world, &mut input, &mut retained, &repeat)? != UpdateKind::Motion {
                return Err("unexpected rebuild while repeating an accepted frame".into());
            }
            controls.insert(
                "retained_repeat".into(),
                compare(&rgba, &repeat.read(renderer)?, frame.width),
            );
            for (name, selection) in [
                ("nonretained_clustered", LocalLightSelection::Clustered),
                ("nonretained_all_lights", LocalLightSelection::All),
            ] {
                let scene = renderer
                    .prepare(input.input())?
                    .with_camera(&label)?
                    .with_local_light_selection(selection);
                let png = renderer.screenshot_scene_png(&scene, frame.width, frame.height)?;
                let (w, h, pixels) = decode(&png)?;
                if (w, h) != (frame.width, frame.height) {
                    return Err("control screenshot size mismatch".into());
                }
                controls.insert(name.into(), compare(&rgba, &pixels, frame.width));
            }
            record["controls"] = Value::Object(controls);
        }
        renderer.check_device()?;
        records.push(record);
    }
    Ok(())
}

fn submit(
    renderer: &Renderer,
    world: &NativeWorld,
    input: &mut NativeRenderInput,
    retained: &mut RetainedScene,
    target: &Target,
) -> Result<UpdateKind, Error> {
    let mut replacement = None;
    let view = world.spatial_view()?;
    let kind = input.try_sync(&view, |scene, label, kind| -> Result<UpdateKind, Error> {
        let output = RetainedTarget {
            view: &target.view,
            format: FORMAT,
            width: target.width,
            height: target.height,
            viewport: None,
        };
        match kind {
            UpdateKind::Motion => {
                retained.submit(renderer, scene, output)?;
            }
            UpdateKind::Rebuilt => {
                // Binding/topology changes need a replacement; its first frame is accepted here.
                let mut fresh = renderer.prepare_retained(scene, label)?;
                fresh.submit(renderer, scene, output)?;
                replacement = Some(fresh);
            }
        }
        Ok(kind)
    })?;
    if let Some(fresh) = replacement {
        *retained = fresh;
    }
    Ok(kind)
}

/// Lights as the renderer packs them, and the storage allocations that implies.
/// Derived from the documented allocation rules; not read back from the GPU.
fn lights(input: &SceneInput, width: u32, height: u32) -> Value {
    let (mut directional, mut shadowed, mut point, mut spot) = (0u32, 0u32, 0u32, 0u32);
    for node in &input.nodes {
        match &node.light {
            Some(SceneLight::Directional(light)) => {
                directional += 1;
                shadowed += u32::from(light.shadows.is_some());
            }
            Some(SceneLight::Point(_)) => point += 1,
            Some(SceneLight::Spot(_)) => spot += 1,
            None => {}
        }
    }
    let local = point + spot;
    let preview = directional == 0 && local == 0;
    let packed = if preview { 1 } else { directional + local };
    let clustered = local > 0;
    let grid = if clustered {
        [width.div_ceil(64), height.div_ceil(64)]
    } else {
        [1, 1]
    };
    let words = if clustered { local.div_ceil(32) } else { 1 };
    let counts = u64::from(grid[0] * grid[1] * 24) * 4;
    json!({"directional": directional, "shadowed_directional": shadowed, "point": point, "spot": spot,
        "local": local, "preview_key": preview, "clustered": clustered, "grid": [grid[0], grid[1], 24],
        "mask_words": words, "derived_lengths": {"lights_bytes": u64::from(packed) * 64,
            "counts_bytes": counts, "masks_bytes": counts * u64::from(words)}})
}

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}
impl Target {
    fn new(renderer: &Renderer, width: u32, height: u32) -> Self {
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("0064 material review frame"),
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
        Self {
            texture,
            view,
            width,
            height,
        }
    }
    fn read(&self, renderer: &Renderer) -> Result<Vec<u8>, Error> {
        let stride = (self.width * 4).div_ceil(256) * 256;
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("0064 material review readback"),
            size: u64::from(stride) * u64::from(self.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        renderer.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })?;
        rx.recv_timeout(Duration::from_secs(10))??;
        let data = buffer.slice(..).get_mapped_range();
        let mut pixels = Vec::with_capacity((self.width * self.height * 4) as usize);
        for row in data.chunks(stride as usize) {
            pixels.extend_from_slice(&row[..(self.width * 4) as usize]);
        }
        drop(data);
        buffer.unmap();
        Ok(pixels)
    }
}

fn describe(rgba: &[u8], width: u32) -> Value {
    let row_bytes = width as usize * 4;
    let rows: Vec<String> = rgba
        .chunks(row_bytes)
        .map(|row| hex(&Sha256::digest(row)[..8]))
        .collect();
    let mut sums = [0u64; 3];
    let mut black = 0u64;
    for p in rgba.chunks_exact(4) {
        for c in 0..3 {
            sums[c] += u64::from(p[c]);
        }
        black += u64::from(p[..3] == [0, 0, 0]);
    }
    let n = (rgba.len() / 4) as f64;
    json!({"rgba_sha256": hex(&Sha256::digest(rgba)), "row_hashes": rows,
        "mean_rgb": sums.map(|s| s as f64 / n), "black_pixels": black})
}

fn compare(a: &[u8], b: &[u8], width: u32) -> Value {
    if a.len() != b.len() {
        return json!({"error": "size mismatch"});
    }
    let (mut differing, mut over_one, mut worst) = (0u64, 0u64, 0u8);
    let mut bbox = [u32::MAX, u32::MAX, 0, 0];
    for (i, (pa, pb)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        let delta = pa
            .iter()
            .zip(pb)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if delta > 0 {
            let (x, y) = (i as u32 % width, i as u32 / width);
            differing += 1;
            over_one += u64::from(delta > 1);
            worst = worst.max(delta);
            bbox = [
                bbox[0].min(x),
                bbox[1].min(y),
                bbox[2].max(x),
                bbox[3].max(y),
            ];
        }
    }
    json!({"rgba_sha256": hex(&Sha256::digest(b)), "differing_pixels": differing,
        "pixels_over_one": over_one, "max_channel_delta": worst,
        "bbox": if differing > 0 { json!(bbox) } else { Value::Null }})
}

fn merge(record: &mut Value, more: Value) {
    if let (Value::Object(a), Value::Object(b)) = (record, more) {
        a.extend(b);
    }
}

fn decode(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), Error> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info()?;
    let size = reader
        .output_buffer_size()
        .ok_or("PNG output size overflow")?;
    let mut pixels = vec![0; size];
    let info = reader.next_frame(&mut pixels)?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err("control screenshot is not 8-bit RGBA".into());
    }
    pixels.truncate(info.buffer_size());
    Ok((info.width, info.height, pixels))
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), Error> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn id(text: &str) -> Result<StableId, Error> {
    if text.len() != 32 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("ID must be exactly 32 hexadecimal digits of the cooked stable ID".into());
    }
    Ok(StableId(u128::from_str_radix(text, 16)?.to_be_bytes()))
}

fn read(path: &Path, limit: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err("input size limit".into());
    }
    Ok(bytes)
}
