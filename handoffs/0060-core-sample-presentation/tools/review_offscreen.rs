//! 0060 offscreen look-dev for the Core Sample (adapted from the 0057 harness;
//! copied into incant_render_gpu_host/examples only while it runs, see
//! offscreen.sh).
//!
//! Drives an ordinary cooked package through HostedOfflineGameSession,
//! NativeRenderHost and the GpuRenderConsumer into an Rgba8UnormSrgb target of
//! `viewport x scale` pixels, following a JSON step plan (held keys for N ticks,
//! resizes, captures). The guest is a logging stand-in (authored pose), a CommonJS
//! development probe, or the package's own cooked program (`cooked`: the exact
//! bytecode and bindings the player embeds, included from its behavior.rs).
//! Each capture records the frame, UI counters and widget geometry.
//! No window, OS input/IME, presentation or performance claim.
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::{InputActions, InputEvent, KeyCode};
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_data::UiRole;
use incant_render_game::{NativeRenderHost, NativeRenderSettings, UiSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider};
use incant_runtime::{CookedScene, StableId};
use incant_script::CompiledScript;
use incant_ui::UiEvent;
use serde_json::{Value, json};
use std::{cell::RefCell, error::Error, path::Path, rc::Rc, sync::Arc, time::Duration};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
mod cooked {
    include!("zz_0060_data/behavior.rs");
}
type Game = HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer<Targets>>>;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[derive(Debug, thiserror::Error)]
#[error("review target unavailable")]
struct TargetError;
/// The harness swaps the shared texture when a step resizes the viewport.
struct Targets(Rc<RefCell<wgpu::Texture>>);
struct Frame(wgpu::TextureView, [u32; 2]);
impl TargetProvider for Targets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> std::result::Result<Frame, TargetError> {
        let texture = self.0.borrow();
        Ok(Frame(
            texture.create_view(&Default::default()),
            [texture.width(), texture.height()],
        ))
    }
}
impl RenderFrame for Frame {
    type Error = TargetError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.0,
            format: FORMAT,
            width: self.1[0],
            height: self.1[1],
            viewport: None,
        }
    }
    fn present(self) -> std::result::Result<(), TargetError> {
        Ok(())
    }
}

const DIGITS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
fn ulid(text: &str) -> Result<StableId> {
    let value = text.bytes().try_fold(0u128, |n, c| {
        let digit = DIGITS.iter().position(|d| *d == c).ok_or("ULID digit")?;
        n.checked_mul(32)
            .map(|n| n + digit as u128)
            .ok_or("ULID range")
    })?;
    Ok(StableId(value.to_be_bytes()))
}
fn ulid_text(id: StableId) -> String {
    let value = u128::from_be_bytes(id.0);
    (0..26)
        .rev()
        .map(|i| DIGITS[((value >> (5 * i)) & 31) as usize] as char)
        .collect()
}
fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(format!("{} exceeds {limit} bytes", path.display()).into());
    }
    Ok(bytes)
}
fn texture(renderer: &Renderer, size: [u32; 2]) -> wgpu::Texture {
    renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("0060 offscreen review target"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
fn pixels(renderer: &Renderer, texture: &wgpu::Texture) -> Result<Vec<u8>> {
    let [w, h] = [texture.width(), texture.height()];
    let stride = (w * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("0060 review readback"),
        size: u64::from(stride) * u64::from(h),
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
                rows_per_image: Some(h),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| drop(tx.send(r)));
    renderer.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })?;
    rx.recv_timeout(Duration::from_secs(10))??;
    let mapped = buffer.slice(..).get_mapped_range();
    let bytes = mapped
        .chunks(stride as usize)
        .flat_map(|row| row[..w as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    Ok(bytes)
}
fn write_png(path: &Path, rgba: &[u8], size: [u32; 2]) -> Result<()> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let mut encoder = png::Encoder::new(file, size[0], size[1]);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

/// Every drawn widget's logical geometry, its accepted edit state and where
/// the renderer would draw its caret.
fn geometry(game: &Game) -> Result<Value> {
    let host = game.host().map_err(|e| e.to_string())?;
    let input = host.ui_input().ok_or("UI input")?;
    let widgets: Vec<_> = input
        .widgets
        .iter()
        .map(|w| {
            json!({"id": ulid_text(w.id), "role": match w.role {
                UiRole::Label => "label", UiRole::Button => "button", UiRole::TextInput => "text_input"},
                "disabled": w.disabled, "focused": w.focused, "pressed": w.pressed,
                "bounds": w.bounds, "visible": w.visible, "origin": w.origin, "clip": w.clip,
                "source": w.layout.source(),
                "lines": w.layout.lines().iter().map(|l| json!({"top": l.top, "height": l.height,
                    "baseline": l.baseline})).collect::<Vec<_>>(),
                "glyph_x": w.layout.glyphs().iter().map(|g| [g.position[0], g.position[0] + g.advance])
                    .collect::<Vec<_>>(),
                "edit": w.edit.as_ref().map(|e| json!({
                    "selection": e.selection.map(|s| [s.anchor, s.focus]),
                    "preedit": e.preedit.as_ref().map(|p| [p.start, p.end])})),
                "caret": incant_render::ui_caret(w).map(|c| json!({"rect": c.rect, "visible": c.visible})),
            })
        })
        .collect();
    Ok(json!({"viewport": input.viewport, "widgets": widgets}))
}

/// The package's cooked clip table, loaded exactly as the player's startup does.
fn clips(data: &Path) -> Result<Vec<incant_audio_runtime::ClipBinding>> {
    let table = read(&data.join("clips.bin"), 12 + 128 * 48)?;
    if table.len() < 12 || &table[..8] != b"NCLIPS01" {
        return Err("clip table header".into());
    }
    let count = u32::from_le_bytes(table[8..12].try_into()?) as usize;
    if table.len() != 12 + count * 48 {
        return Err("clip table length".into());
    }
    let mut out = Vec::with_capacity(count);
    for row in table[12..].chunks(48) {
        let id = StableId(row[..16].try_into()?);
        let key: String = row[16..].iter().map(|b| format!("{b:02x}")).collect();
        out.push(incant_audio_runtime::ClipBinding::load(
            id,
            &data.join("audio"),
            &key,
        )?);
    }
    Ok(out)
}

fn key(name: &Value) -> Result<KeyCode> {
    Ok(serde_json::from_value(name.clone())?)
}

fn run_case(
    renderer: &Arc<Renderer>,
    data: &Path,
    out: &Path,
    case: &Value,
    guest_js: Option<&str>,
) -> Result<Value> {
    let name = case["name"].as_str().ok_or("case name")?;
    let scale = case["scale"].as_f64().ok_or("case scale")? as f32;
    let mut viewport: [f32; 2] = serde_json::from_value(case["viewport"].clone())?;
    let manifest: Value = serde_json::from_slice(&read(&data.join("package.json"), 8192)?)?;
    let hex = manifest["camera"].as_str().ok_or("package camera")?;
    let mut camera = [0u8; 16];
    for (i, byte) in camera.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2).ok_or("camera hex")?, 16)?;
    }
    let scene = CookedScene::from_bytes(&read(&data.join("world.scene"), 128 * 1024 * 1024)?)?;
    let actions: InputActions =
        serde_json::from_slice(&read(&data.join("input-actions.json"), 256 * 1024)?)?;
    let size = |viewport: [f32; 2]| viewport.map(|v| (v * scale).round() as u32);
    let shared = Rc::new(RefCell::new(texture(renderer, size(viewport))));
    let guest = match guest_js {
        Some("cooked") => cooked::load()?,
        Some(source) => CompiledScript::compile_named("gameplay.js", source)?,
        None => CompiledScript::compile(
            "exports.default={update({input,log}){for(const e of input.ui.events)log(e.kind+' '+e.entity_id+('value' in e?' '+e.value:''));}};",
        )?,
    };
    let mut game = Game::new(
        scene,
        guest,
        actions,
        clips(data)?,
        48000,
        NativeRenderSettings {
            ui: Some(UiSettings::new(viewport)),
            text: Some(incant_render_native::TextSettings::new(data.join("fonts"))),
            model_cache: data.join("models"),
            texture_cache: data.join("textures"),
            camera: StableId(camera),
            consumer: GpuRenderSettings {
                renderer: Arc::clone(renderer),
                targets: Targets(Rc::clone(&shared)),
            },
        },
    )
    .map_err(|e| e.to_string())?;
    let startup = renderer.ui_uploads();
    let mut captures = Vec::new();
    for step in case["steps"].as_array().ok_or("case steps")? {
        let host = game.host().map_err(|e| e.to_string())?;
        let ui = host.ui().ok_or("UI owner")?;
        let mut physical = Vec::new();
        let mut receipts = Vec::new();
        if let Some(pairs) = step.get("keys").and_then(Value::as_array) {
            for pair in pairs {
                physical.push(InputEvent::Key {
                    code: key(&pair[0])?,
                    down: pair[1].as_bool().ok_or("key state")?,
                });
            }
        }
        if let Some(events) = step.get("events").and_then(Value::as_array) {
            for event in events {
                physical.push(serde_json::from_value::<InputEvent>(event.clone())?);
            }
        }
        if let Some(id) = step.get("focus").and_then(Value::as_str) {
            receipts.push(UiEvent::Focus(ulid(id)?));
        }
        if let Some(pointer) = step.get("pointer").and_then(Value::as_array) {
            let input = host.ui_input().ok_or("UI input")?;
            let id = ulid(pointer[0].as_str().ok_or("pointer id")?)?;
            let widget = input
                .widgets
                .iter()
                .find(|w| w.id == id)
                .ok_or("pointer widget")?;
            let [l, t, r, b] = widget.visible;
            receipts.push(UiEvent::Pointer {
                position: [(l + r) / 2., (t + b) / 2.],
                pressed: pointer[1].as_bool().ok_or("pointer state")?,
                target: ui.pointer_token().ok_or("pointer token")?,
            });
        }
        if let Some(text) = step.get("commit").and_then(Value::as_str) {
            receipts.push(UiEvent::Commit {
                target: ui.focus().ok_or("focus")?,
                text: text.into(),
            });
        }
        if let Some(preedit) = step.get("preedit").and_then(Value::as_array) {
            let cursor = preedit[1]
                .as_array()
                .map(|r| -> Result<_> {
                    Ok(r[0].as_u64().ok_or("cursor")? as usize
                        ..r[1].as_u64().ok_or("cursor")? as usize)
                })
                .transpose()?;
            receipts.push(UiEvent::Preedit {
                target: ui.focus().ok_or("focus")?,
                text: preedit[0].as_str().ok_or("preedit text")?.into(),
                cursor,
            });
        }
        if let Some(resize) = step.get("resize") {
            viewport = serde_json::from_value(resize.clone())?;
            *shared.borrow_mut() = texture(renderer, size(viewport));
            game.resize_ui(viewport).map_err(|e| e.to_string())?;
        }
        let ticks = step
            .get("ticks")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .max(1);
        for tick in 0..ticks {
            let mut pcm = vec![0.; game.next_output_frames().map_err(|e| e.to_string())? * 2];
            let (events, ui_events): (&[InputEvent], &[UiEvent]) = if tick == 0 {
                (&physical, &receipts)
            } else {
                (&[], &[])
            };
            game.tick_with_ui(events, ui_events, &mut pcm)
                .map_err(|e| e.to_string())?;
        }
        let Some(label) = step.get("capture").and_then(Value::as_str) else {
            continue;
        };
        let target = shared.borrow().clone();
        let rgba = pixels(renderer, &target)?;
        let file = out.join(format!("{name}-{label}.png"));
        write_png(&file, &rgba, size(viewport))?;
        let host = game.host().map_err(|e| e.to_string())?;
        let ui = host.ui().ok_or("UI owner")?;
        let fields: serde_json::Map<_, _> = ui
            .widgets()
            .iter()
            .filter_map(|w| {
                ui.text(w.id).map(|buffer| {
                    (ulid_text(w.id), json!({"text": buffer.text(), "display": buffer.display_text(),
                        "selection": buffer.display_selection().map(|s| [s.anchor, s.focus]),
                        "preedit": buffer.preedit().map(|p| [p.display_range.start, p.display_range.end])}))
                })
            })
            .collect();
        let stats = host.consumer().ui_stats().ok_or("UI stats")?;
        let scene = host.consumer().stats().ok_or("scene stats")?;
        let uploads = renderer.ui_uploads();
        captures.push(json!({
            "capture": file.file_name().and_then(|n| n.to_str()),
            "png_rgba_sha256": incant_assets_runtime::sha256(&rgba), "size": size(viewport),
            "tick": game.world().map_err(|e| e.to_string())?.tick(),
            "focus": ui.focus().map(|f| ulid_text(f.widget())),
            "fields": fields,
            "scene": {"model_entities": scene.model_entities, "primitive_instances": scene.primitive_instances,
                "model_triangles": scene.model_triangles, "model_draw_calls": scene.model_draw_calls,
                "diagnostic_entities": scene.diagnostic_entities},
            "ui": {"widgets": stats.widgets, "glyph_quads": stats.glyph_quads, "quads": stats.quads,
                "missing_glyphs": stats.missing_glyphs, "gpu_bytes": stats.gpu_bytes},
            "uploads_since_startup": {
                "atlas_textures": uploads.atlas_textures - startup.atlas_textures,
                "quad_buffers": uploads.quad_buffers - startup.quad_buffers,
                "frame_writes": uploads.frame_writes - startup.frame_writes},
            "geometry": geometry(&game)?,
        }));
    }
    let logs: Vec<_> = game
        .logs()
        .map_err(|e| e.to_string())?
        .entries()
        .iter()
        .map(|e| json!({"tick": e.tick, "message": e.message}))
        .collect();
    let host = game.host().map_err(|e| e.to_string())?;
    Ok(
        json!({"case": name, "scale": scale, "accepted_frames": host.consumer().accepted_frames(),
        "guest_logs": logs, "captures": captures}),
    )
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let (data, plan, out, guest) = match &args[..] {
        [data, plan, out] => (data, plan, out, None),
        [data, plan, out, guest] if guest == "cooked" => {
            (data, plan, out, Some("cooked".to_string()))
        }
        [data, plan, out, guest] => (data, plan, out, Some(std::fs::read_to_string(guest)?)),
        _ => return Err(
            "usage: zz_0060_review <package-data> <plan.json> <new-output-directory> [guest.js]"
                .into(),
        ),
    };
    let (data, out) = (Path::new(data), Path::new(out));
    let plan: Value = serde_json::from_slice(&read(Path::new(plan), 1 << 20)?)?;
    std::fs::create_dir(out)?;
    let renderer = Arc::new(Renderer::headless().map_err(|e| e.to_string())?);
    let adapter = renderer.adapter.get_info();
    let mut cases = Vec::new();
    for case in plan["cases"].as_array().ok_or("plan cases")? {
        cases.push(run_case(&renderer, data, out, case, guest.as_deref())?);
    }
    let report = json!({"format": "incant-0060-offscreen-review", "adapter": adapter.name,
        "guest": match guest.as_deref() { Some("cooked") => "package cooked program", Some(_) => "development probe script", None => "logging stand-in" },
        "backend": format!("{:?}", adapter.backend), "target_format": format!("{FORMAT:?}"),
        "cases": cases});
    std::fs::write(
        out.join("review.json"),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    println!("{}", out.join("review.json").display());
    Ok(())
}
