//! Offscreen review helper for an authored HUD: one cooked package, the program
//! this build embedded for it, the composed game and the GpuRenderConsumer with
//! its screen-space UI layer. Replays an input recording and writes raw RGBA
//! frames at chosen ticks, each with the engine's current HUD label text,
//! flipbook frames and logs. No window, OS input, presentation or performance
//! claim; output I/O is not an atomic transaction.
//!
//! Build with `INCANT_NATIVE_PROJECT=<project.json>` and
//! `--features project,render-gpu --example hud_frames`, then run
//! `hud_frames <package-data> <recording.json> <WIDTHxHEIGHT> <scale> <ticks,...> <new-output>`.
#[cfg(feature = "render-gpu")]
mod review {
    use incant_game_runtime::HostedOfflineGameSession;
    use incant_input::{InputActions, InputReplay};
    use incant_render::{Renderer, RetainedTarget, wgpu};
    use incant_render_game::{NativeRenderHost, NativeRenderSettings, UiSettings};
    use incant_render_gpu_host::{
        GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider,
    };
    use incant_runtime::{CookedScene, StableId};
    use serde_json::json;
    use std::{io::Read, path::Path, sync::Arc, time::Duration};

    mod embedded {
        include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
    }

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
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

    fn read(path: &Path, limit: u64) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?
            .take(limit + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(format!("{} exceeds {limit} bytes", path.display()).into());
        }
        Ok(bytes)
    }
    fn hex(id: StableId) -> String {
        id.0.iter().map(|b| format!("{b:02x}")).collect()
    }
    fn pixels(renderer: &Renderer, texture: &wgpu::Texture) -> Result<Vec<u8>> {
        let [w, h] = [texture.width(), texture.height()];
        let stride = (w * 4).div_ceil(256) * 256;
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("HUD review readback"),
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

    /// Current label/button text held by the UI owner, keyed by entity ID.
    fn hud(game: &Game) -> Result<serde_json::Value> {
        let host = game.host().map_err(|e| e.to_string())?;
        let ui = host.ui().ok_or("this package has no UI owner")?;
        let texts: serde_json::Map<_, _> = ui
            .widgets()
            .iter()
            .filter_map(|w| {
                w.node.kind.text().map(|t| {
                    (
                        hex(w.id),
                        json!({"text": t.content.as_str(), "parent": w.parent.map(hex)}),
                    )
                })
            })
            .collect();
        let stats = host.consumer().ui_stats().ok_or("UI stats")?;
        Ok(
            json!({"labels": texts, "widgets": stats.widgets, "glyph_quads": stats.glyph_quads,
            "missing_glyphs": stats.missing_glyphs, "gpu_bytes": stats.gpu_bytes}),
        )
    }

    pub fn main() -> Result<()> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        let [data, recording, viewport, scale, ticks, out] = &args[..] else {
            return Err("usage: hud_frames <package-data> <recording.json> <WIDTHxHEIGHT> <scale> <ticks,...> <new-output>".into());
        };
        let (data, out) = (Path::new(data), Path::new(out));
        let (w, h) = viewport.split_once('x').ok_or("viewport is WIDTHxHEIGHT")?;
        let viewport = [w.parse::<f32>()?, h.parse::<f32>()?];
        let scale: f32 = scale.parse()?;
        let mut ticks = ticks
            .split(',')
            .map(str::parse::<u64>)
            .collect::<std::result::Result<Vec<_>, _>>()?;
        ticks.sort_unstable();
        ticks.dedup();
        let size = viewport.map(|v| (v * scale).round() as u32);
        if viewport.iter().any(|v| !(16.0..=4096.0).contains(v))
            || !(1.0..=4.0).contains(&scale)
            || size.iter().any(|v| *v > 4096)
            || ticks.is_empty()
            || ticks[0] == 0
            || ticks.len() > 64
            || *ticks.last().unwrap() > 36_000
        {
            return Err("viewport 16..4096, scale 1..4, 1..64 capture ticks in 1..36000".into());
        }
        let manifest: serde_json::Value =
            serde_json::from_slice(&read(&data.join("package.json"), 8192)?)?;
        let camera = manifest["camera"].as_str().ok_or("package camera")?;
        let mut id = [0u8; 16];
        for (i, byte) in id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(camera.get(i * 2..i * 2 + 2).ok_or("camera hex")?, 16)?;
        }
        let scene = CookedScene::from_bytes(&read(&data.join("world.scene"), 128 << 20)?)?;
        let actions: InputActions =
            serde_json::from_slice(&read(&data.join("input-actions.json"), 256 << 10)?)?;
        let replay = InputReplay::from_text(
            std::str::from_utf8(&read(
                Path::new(recording),
                incant_input::MAX_RECORDING_BYTES as u64,
            )?)?,
            0,
            scene.tick_rate(),
        )?;
        if replay.has_ui() || *ticks.last().unwrap() > replay.end_tick() {
            return Err("recording must cover every tick and carry no UI receipts".into());
        }
        let renderer = Arc::new(Renderer::headless().map_err(|e| e.to_string())?);
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("HUD review target"),
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
        let mut game = Game::new(
            scene,
            embedded::load()?,
            actions,
            vec![],
            48_000,
            NativeRenderSettings {
                ui: Some(UiSettings::new(viewport)),
                text: Some(incant_render_native::TextSettings::new(data.join("fonts"))),
                model_cache: data.join("models"),
                texture_cache: data.join("textures"),
                camera: StableId(id),
                consumer: GpuRenderSettings {
                    renderer: Arc::clone(&renderer),
                    targets: Targets(texture.clone()),
                },
            },
        )
        .map_err(|e| e.to_string())?;
        std::fs::create_dir(out)?;
        let mut captures = Vec::new();
        for tick in 1..=*ticks.last().unwrap() {
            let mut pcm = vec![0.; game.next_output_frames().map_err(|e| e.to_string())? * 2];
            game.tick_with_ui(replay.events_at(tick)?, &[], &mut pcm)
                .map_err(|e| e.to_string())?;
            if ticks.binary_search(&tick).is_err() {
                continue;
            }
            let rgba = pixels(&renderer, &texture)?;
            let name = format!("tick-{tick:05}.rgba");
            std::fs::write(out.join(&name), &rgba)?;
            let world = game.world().map_err(|e| e.to_string())?;
            let flipbooks: Vec<_> = world
                .spatial_view()?
                .nodes()
                .filter_map(|n| {
                    n.flipbook.map(
                        |s| json!({"id": hex(n.id), "frame": s.frame(), "playing": s.playing()}),
                    )
                })
                .collect();
            captures.push(json!({"tick": world.tick(), "rgba": name, "size": size,
                "rgba_sha256": incant_assets_runtime::sha256(&rgba), "hud": hud(&game)?,
                "flipbooks": flipbooks}));
        }
        let logs: Vec<_> = game
            .logs()
            .map_err(|e| e.to_string())?
            .entries()
            .iter()
            .map(|e| json!({"tick": e.tick, "message": e.message}))
            .collect();
        let adapter = renderer.adapter.get_info();
        let report = json!({"format": "incant-0061-hud-frames", "adapter": adapter.name,
            "backend": format!("{:?}", adapter.backend), "target_format": format!("{FORMAT:?}"),
            "viewport": viewport, "scale": scale, "size": size, "captures": captures, "logs": logs,
            "scope": "offscreen GPU frames of the embedded program; no window or presentation"});
        std::fs::write(
            out.join("report.json"),
            serde_json::to_string_pretty(&report)? + "\n",
        )?;
        println!("{}", out.join("report.json").display());
        Ok(())
    }
}

#[cfg(feature = "render-gpu")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    review::main()
}

#[cfg(not(feature = "render-gpu"))]
fn main() {
    eprintln!("hud_frames requires --features render-gpu (with project for an authored package)");
    std::process::exit(2);
}
