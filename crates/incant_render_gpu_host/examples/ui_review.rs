//! Bounded offscreen review helper: an ordinary cooked package (scene, model and
//! font caches) driven through the composed game, NativeRenderHost and the
//! GpuRenderConsumer at chosen logical sizes and pixel ratios, with routed keys
//! and explicit UI receipts. Writes PNG frames and each step's accepted UI
//! snapshot. A logging guest stands in for the package's embedded program (the
//! review fixture has no Script). No window, OS input/IME, presentation or
//! performance claim; output I/O is not an atomic transaction.
use incant_game_runtime::HostedOfflineGameSession;
use incant_input::{InputActions, InputEvent, KeyCode};
use incant_render::{Renderer, RetainedTarget, wgpu};
use incant_render_game::{NativeRenderHost, NativeRenderSettings, UiSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider};
use incant_runtime::{CookedScene, StableId};
use incant_script::CompiledScript;
use incant_ui::UiEvent;
use serde_json::json;
use std::{error::Error, path::Path, sync::Arc, time::Duration};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Game = HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer<Targets>>>;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[derive(Debug, thiserror::Error)]
#[error("review target unavailable")]
struct TargetError;
struct Targets(wgpu::Texture);
struct Frame(wgpu::TextureView, [u32; 2]);
impl TargetProvider for Targets {
    type Error = TargetError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> std::result::Result<Frame, TargetError> {
        Ok(Frame(
            self.0.create_view(&Default::default()),
            [self.0.width(), self.0.height()],
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

/// Crockford base32 ULID text to its 128-bit stable ID.
fn ulid(text: &str) -> Result<StableId> {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let value = text.bytes().try_fold(0u128, |n, c| {
        let digit = DIGITS.iter().position(|d| *d == c).ok_or("ULID digit")?;
        n.checked_mul(32)
            .map(|n| n + digit as u128)
            .ok_or("ULID range")
    })?;
    Ok(StableId(value.to_be_bytes()))
}
fn ulid_text(id: StableId) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
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

enum Step {
    Keys(Vec<(KeyCode, bool)>),
    Focus(&'static str),
    Pointer(&'static str, bool),
    Commit(&'static str),
    Preedit(&'static str, Option<std::ops::Range<usize>>),
}
use Step::*;
fn keys(pairs: &[(KeyCode, bool)]) -> Step {
    Keys(pairs.to_vec())
}
const OPTIONS: &str = "00000000000000000000000025";
const NAME: &str = "0000000000000000000000002A";
const ARABIC: &str = "0000000000000000000000002C";
const CJK: &str = "0000000000000000000000002E";

/// Review steps; a name captures the frame after that step's tick.
fn script(full: bool) -> Vec<(Step, Option<&'static str>)> {
    let tab = |down| (KeyCode::Tab, down);
    let mut select = vec![(KeyCode::ShiftLeft, true)];
    for _ in 0.."Lovelace".chars().count() {
        select.extend([(KeyCode::ArrowLeft, true), (KeyCode::ArrowLeft, false)]);
    }
    select.push((KeyCode::ShiftLeft, false));
    let mut steps = vec![
        (keys(&[]), Some("01-idle")),
        (keys(&[tab(true)]), None),
        (keys(&[tab(false)]), Some("02-focus-resume")),
    ];
    if full {
        steps.extend([
            (Pointer(OPTIONS, true), Some("03-pressed-options")),
            (Pointer(OPTIONS, false), None),
            (Focus(NAME), Some("04-field-caret")),
        ]);
    } else {
        steps.push((Focus(NAME), None));
    }
    steps.push((Keys(select), Some("05-field-selection")));
    if full {
        steps.extend([
            (Commit("Byron"), Some("06-field-value")),
            (Focus(ARABIC), Some("07-arabic-caret")),
        ]);
    }
    steps.extend([
        (Focus(CJK), None),
        (Preedit("語", Some(3..3)), Some("08-cjk-preedit")),
    ]);
    if full {
        steps.extend([
            (Commit("語"), Some("09-cjk-commit")),
            (keys(&[(KeyCode::Enter, true)]), None),
            (keys(&[(KeyCode::Enter, false)]), None),
        ]);
    }
    steps
}

fn pixels(renderer: &Renderer, texture: &wgpu::Texture) -> Result<Vec<u8>> {
    let [w, h] = [texture.width(), texture.height()];
    let stride = (w * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("UI review readback"),
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
fn sha256(bytes: &[u8]) -> String {
    incant_assets_runtime::sha256(bytes)
}

fn run_case(
    renderer: &Arc<Renderer>,
    data: &Path,
    out: &Path,
    name: &str,
    viewport: [f32; 2],
    scale: f32,
    full: bool,
) -> Result<serde_json::Value> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&read(&data.join("package.json"), 8192)?)?;
    let hex = manifest["camera"].as_str().ok_or("package camera")?;
    let mut camera = [0u8; 16];
    for (i, byte) in camera.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2).ok_or("camera hex")?, 16)?;
    }
    let scene = CookedScene::from_bytes(&read(&data.join("world.scene"), 128 * 1024 * 1024)?)?;
    let actions: InputActions =
        serde_json::from_slice(&read(&data.join("input-actions.json"), 256 * 1024)?)?;
    let size = viewport.map(|v| (v * scale).round() as u32);
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("UI review target"),
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
    });
    let guest = CompiledScript::compile(
        "exports.default={update({input,log}){for(const e of input.ui.events)log(e.kind+' '+e.entity_id+('value' in e?' '+e.value:''));}};",
    )?;
    let mut game = Game::new(
        scene,
        guest,
        actions,
        vec![],
        48000,
        NativeRenderSettings {
            ui: Some(UiSettings::new(viewport)),
            text: Some(incant_render_native::TextSettings::new(data.join("fonts"))),
            model_cache: data.join("models"),
            texture_cache: data.join("textures"),
            camera: StableId(camera),
            consumer: GpuRenderSettings {
                renderer: Arc::clone(renderer),
                targets: Targets(texture.clone()),
            },
        },
    )
    .map_err(|e| e.to_string())?;
    let startup = renderer.ui_uploads();
    let mut captures = Vec::new();
    let mut pointers = Vec::new();
    for (step, capture) in script(full) {
        let host = game.host().map_err(|e| e.to_string())?;
        let ui = host.ui().ok_or("UI owner")?;
        let (physical, receipts) = match step {
            Keys(pairs) => (
                pairs
                    .into_iter()
                    .map(|(code, down)| InputEvent::Key { code, down })
                    .collect(),
                vec![],
            ),
            Focus(id) => (vec![], vec![UiEvent::Focus(ulid(id)?)]),
            Pointer(id, pressed) => {
                let input = host.ui_input().ok_or("UI input")?;
                let widget = input
                    .widgets
                    .iter()
                    .find(|w| Ok(w.id) == ulid(id).map_err(|e| e.to_string()))
                    .ok_or("pointer widget")?;
                let [l, t, r, b] = widget.visible;
                let position = [(l + r) / 2., (t + b) / 2.];
                let tick = game.world().map_err(|e| e.to_string())?.tick() + 1;
                pointers.push(json!({"tick": tick, "position": position, "pressed": pressed}));
                (
                    vec![],
                    vec![UiEvent::Pointer {
                        position,
                        pressed,
                        target: ui.pointer_token().ok_or("pointer token")?,
                    }],
                )
            }
            Commit(text) => (
                vec![],
                vec![UiEvent::Commit {
                    target: ui.focus().ok_or("focus")?,
                    text: text.into(),
                }],
            ),
            Preedit(text, cursor) => (
                vec![],
                vec![UiEvent::Preedit {
                    target: ui.focus().ok_or("focus")?,
                    text: text.into(),
                    cursor,
                }],
            ),
        };
        let mut pcm = vec![0.; game.next_output_frames().map_err(|e| e.to_string())? * 2];
        game.tick_with_ui(&physical, &receipts, &mut pcm)
            .map_err(|e| e.to_string())?;
        let Some(label) = capture else { continue };
        let rgba = pixels(renderer, &texture)?;
        let file = out.join(format!("{name}-{label}.png"));
        write_png(&file, &rgba, size)?;
        let host = game.host().map_err(|e| e.to_string())?;
        let ui = host.ui().ok_or("UI owner")?;
        let values: serde_json::Map<_, _> = [NAME, ARABIC, CJK]
            .iter()
            .map(|id| -> Result<_> {
                let buffer = ui.text(ulid(id)?).ok_or("field buffer")?;
                Ok((
                    (*id).to_owned(),
                    json!({"text": buffer.text(), "display": buffer.display_text(),
                        "selection": buffer.display_selection().map(|s| [s.anchor, s.focus]),
                        "preedit": buffer.preedit().map(|p| [p.display_range.start, p.display_range.end])}),
                ))
            })
            .collect::<Result<_>>()?;
        let stats = host.consumer().ui_stats().ok_or("UI stats")?;
        let uploads = renderer.ui_uploads();
        captures.push(json!({
            "capture": file.file_name().and_then(|n| n.to_str()),
            "png_rgba_sha256": sha256(&rgba), "size": size,
            "tick": game.world().map_err(|e| e.to_string())?.tick(),
            "accepted_ui_snapshot": game.input().map_err(|e| e.to_string())?.ui,
            "fields": values,
            "ui": {"widgets": stats.widgets, "glyph_quads": stats.glyph_quads, "quads": stats.quads,
                "missing_glyphs": stats.missing_glyphs, "gpu_bytes": stats.gpu_bytes},
            "uploads_since_startup": {
                "atlas_textures": uploads.atlas_textures - startup.atlas_textures,
                "quad_buffers": uploads.quad_buffers - startup.quad_buffers,
                "frame_writes": uploads.frame_writes - startup.frame_writes},
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
    let ui = host.ui().ok_or("UI owner")?;
    let paint = host.ui_paint().ok_or("UI paint")?;
    let values: serde_json::Map<_, _> = ui
        .widgets()
        .iter()
        .filter_map(|w| {
            ui.text(w.id)
                .map(|b| (ulid_text(w.id), serde_json::Value::from(b.text())))
        })
        .collect();
    let last = json!({"tick": game.world().map_err(|e| e.to_string())?.tick(),
        "focus": ui.focus().map(|f| ulid_text(f.widget())), "values": values,
        "glyphs": paint.texts.iter().map(|t| t.glyphs.len()).sum::<usize>(),
        "atlas_sha256": sha256(paint.atlas.pixels())});
    Ok(
        json!({"case": name, "viewport": viewport, "scale": scale, "target": size,
        "accepted_frames": host.consumer().accepted_frames(), "final": last,
        "pointers": pointers, "guest_logs": logs, "captures": captures}),
    )
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [data, out] = &args[..] else {
        return Err("usage: ui_review <package-data-directory> <new-output-directory>".into());
    };
    let (data, out) = (Path::new(data), Path::new(out));
    std::fs::create_dir(out)?;
    let renderer = Arc::new(Renderer::headless().map_err(|e| e.to_string())?);
    let adapter = renderer.adapter.get_info();
    let mut cases = Vec::new();
    for (name, viewport, scale, full) in [
        ("desktop-1x", [960., 540.], 1., true),
        ("desktop-2x", [960., 540.], 2., false),
        ("narrow-2x", [360., 640.], 2., false),
    ] {
        cases.push(run_case(&renderer, data, out, name, viewport, scale, full)?);
    }
    let report = json!({"format": "incant-0056-ui-review", "adapter": adapter.name,
        "backend": format!("{:?}", adapter.backend), "format_target": format!("{FORMAT:?}"),
        "cases": cases});
    std::fs::write(
        out.join("review.json"),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    println!("{}", out.join("review.json").display());
    Ok(())
}
