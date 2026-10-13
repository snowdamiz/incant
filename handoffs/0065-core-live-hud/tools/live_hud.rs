//! 0065 packet-only offscreen capture adapter for the Core Sample live HUD.
//!
//! Copied to `crates/incant_game_player/examples/zz_0065_live_hud.rs` only while
//! `tools/build_variant.py` builds it (`INCANT_NATIVE_PROJECT=<variant>/core.json`,
//! `--features project,render-gpu`); the copy is removed afterwards.
//!
//! Replays one accepted numerical route (the `binding_oracle` recordings) through
//! the program this build embedded, on the composed game with the GpuRenderConsumer
//! offscreen. It follows the ordinary player's offline route: the recording is
//! indexed by host step, a restart request is accepted after the complete tick
//! through the public `HostedOfflineGameSession::restart_from`, and held input
//! crosses that replacement under the fence rules of the player's crate-private
//! `restart/input.rs` (mirrored below). It only reads the public runtime snapshot
//! (UI owner, render input, consumer stats, logs, world tick) and writes raw RGBA
//! frames at requested host steps. It never edits the world, HUD, input or
//! program. No window, device, presentation or performance claim.
//!
//! Usage: zz_0065_live_hud <package-data> <recording.json> <plan.json> <new-output>
#[cfg(feature = "render-gpu")]
mod review {
    use incant_audio_runtime::ClipBinding;
    use incant_game_runtime::{GameSeed, HostedOfflineGameSession};
    use incant_input::{
        GamepadAxis, GamepadButton, InputActions, InputEvent, InputReplay, KeyCode,
        MAX_EVENTS_PER_TICK, MAX_GAMEPADS, MAX_TOUCHES, MouseButton, TouchPhase,
    };
    use incant_render::{Renderer, RetainedTarget, wgpu};
    use incant_render_game::{NativeRenderHost, NativeRenderSettings, UiSettings};
    use incant_render_gpu_host::{
        GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider,
    };
    use incant_runtime::{CookedScene, StableId};
    use incant_script::SessionRequest;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Write as _;
    use std::{
        io::Read,
        path::Path,
        sync::Arc,
        time::{Duration, Instant},
    };

    mod embedded {
        include!(concat!(env!("OUT_DIR"), "/behavior.rs"));
    }

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    type Game = HostedOfflineGameSession<NativeRenderHost<GpuRenderConsumer<Targets>>>;
    const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    /// Packet bounds: retained frames and their pixel size (brief 0065).
    const MAX_FRAMES: usize = 24;
    const MAX_SIZE: [u32; 2] = [960, 540];
    const SAMPLE_RATE: u32 = 48_000;

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

    /// Physical ownership survives game replacement, gameplay values do not.
    /// Mirrors `crates/incant_game_player/src/restart/input.rs`: a control held
    /// across a replacement stays suppressed until its actual release or zero,
    /// and the new game is told which pads are still connected.
    #[derive(Clone, Default)]
    struct Held {
        keys: BTreeSet<KeyCode>,
        mouse: BTreeSet<MouseButton>,
        touches: BTreeSet<u32>,
    }
    #[derive(Default)]
    struct Pad {
        buttons: [bool; 17],
        axes: [bool; 4],
        blocked_buttons: [bool; 17],
        blocked_axes: [bool; 4],
    }
    #[derive(Default)]
    struct Fence {
        held: Held,
        blocked: Held,
        pads: BTreeMap<u32, Pad>,
        announce: bool,
        output: Vec<InputEvent>,
    }
    fn button(b: GamepadButton) -> usize {
        match b {
            GamepadButton::South => 0,
            GamepadButton::East => 1,
            GamepadButton::West => 2,
            GamepadButton::North => 3,
            GamepadButton::LeftShoulder => 4,
            GamepadButton::RightShoulder => 5,
            GamepadButton::LeftTrigger => 6,
            GamepadButton::RightTrigger => 7,
            GamepadButton::Select => 8,
            GamepadButton::Start => 9,
            GamepadButton::Guide => 10,
            GamepadButton::LeftStick => 11,
            GamepadButton::RightStick => 12,
            GamepadButton::DpadUp => 13,
            GamepadButton::DpadDown => 14,
            GamepadButton::DpadLeft => 15,
            GamepadButton::DpadRight => 16,
        }
    }
    fn axis(a: GamepadAxis) -> usize {
        match a {
            GamepadAxis::LeftX => 0,
            GamepadAxis::LeftY => 1,
            GamepadAxis::RightX => 2,
            GamepadAxis::RightY => 3,
        }
    }
    fn digital<T: Ord + Copy>(
        held: &mut BTreeSet<T>,
        blocked: &mut BTreeSet<T>,
        key: T,
        down: bool,
    ) -> bool {
        let suppress = blocked.contains(&key);
        if down {
            held.insert(key);
        } else {
            held.remove(&key);
            blocked.remove(&key);
        }
        !suppress
    }
    impl Fence {
        fn replace_session(&mut self) {
            self.blocked = self.held.clone();
            for pad in self.pads.values_mut() {
                pad.blocked_buttons = pad.buttons;
                pad.blocked_axes = pad.axes;
            }
            self.announce = true;
            self.output.clear();
        }
        fn filter(&mut self, events: &[InputEvent]) -> Result<&[InputEvent]> {
            if events.len() > MAX_EVENTS_PER_TICK {
                return Err("restart input event limit".into());
            }
            self.output.clear();
            if self.announce {
                for id in self.pads.keys() {
                    self.output.push(InputEvent::GamepadConnected { id: *id });
                }
                self.announce = false;
            }
            for event in events {
                let pass = match event {
                    InputEvent::Key { code, down } => {
                        digital(&mut self.held.keys, &mut self.blocked.keys, *code, *down)
                    }
                    InputEvent::MouseButton { button, down } => digital(
                        &mut self.held.mouse,
                        &mut self.blocked.mouse,
                        *button,
                        *down,
                    ),
                    InputEvent::GamepadConnected { id } => {
                        if !self.pads.contains_key(id) && self.pads.len() == MAX_GAMEPADS {
                            return Err("restart gamepad count limit".into());
                        }
                        self.pads.entry(*id).or_default();
                        true
                    }
                    InputEvent::GamepadDisconnected { id } => {
                        self.pads.remove(id);
                        true
                    }
                    InputEvent::GamepadButton {
                        id,
                        button: b,
                        value,
                    } => {
                        if !value.is_finite() || !(0. ..=1.).contains(value) {
                            return Err("restart gamepad value bounds".into());
                        }
                        match self.pads.get_mut(id) {
                            Some(pad) => {
                                let b = button(*b);
                                let suppress = pad.blocked_buttons[b];
                                pad.buttons[b] = *value != 0.;
                                if *value == 0. {
                                    pad.blocked_buttons[b] = false;
                                }
                                !suppress
                            }
                            None => true,
                        }
                    }
                    InputEvent::GamepadAxis { id, axis: a, value } => {
                        if !value.is_finite() || !(-1. ..=1.).contains(value) {
                            return Err("restart gamepad value bounds".into());
                        }
                        match self.pads.get_mut(id) {
                            Some(pad) => {
                                let a = axis(*a);
                                let suppress = pad.blocked_axes[a];
                                pad.axes[a] = *value != 0.;
                                if *value == 0. {
                                    pad.blocked_axes[a] = false;
                                }
                                !suppress
                            }
                            None => true,
                        }
                    }
                    InputEvent::Touch { id, phase, .. } => {
                        let down = !matches!(phase, TouchPhase::Up | TouchPhase::Cancel);
                        if down
                            && !self.held.touches.contains(id)
                            && self.held.touches.len() == MAX_TOUCHES
                        {
                            return Err("restart touch limit".into());
                        }
                        digital(&mut self.held.touches, &mut self.blocked.touches, *id, down)
                    }
                    _ => true,
                };
                if pass {
                    if self.output.len() == MAX_EVENTS_PER_TICK {
                        return Err("restart input event limit".into());
                    }
                    self.output.push(event.clone());
                }
            }
            Ok(&self.output)
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
    fn hex(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
    fn ulid(id: StableId) -> String {
        ulid::Ulid::from_bytes(id.0).to_string()
    }
    /// The package's cooked clip table, loaded as the player's startup does.
    fn clips(data: &Path) -> Result<Vec<ClipBinding>> {
        let table = read(&data.join("clips.bin"), 12 + 128 * 48)?;
        if table.len() < 12 || &table[..8] != b"NCLIPS01" {
            return Err("clip table header".into());
        }
        let count = u32::from_le_bytes(table[8..12].try_into()?) as usize;
        if count > 128 || table.len() != 12 + count * 48 {
            return Err("clip table length/count".into());
        }
        table[12..]
            .chunks(48)
            .map(|row| {
                let id = StableId(row[..16].try_into()?);
                Ok(ClipBinding::load(
                    id,
                    &data.join("audio"),
                    &hex(&row[16..]),
                )?)
            })
            .collect()
    }
    fn pixels(renderer: &Renderer, texture: &wgpu::Texture) -> Result<Vec<u8>> {
        let [w, h] = [texture.width(), texture.height()];
        let stride = (w * 4).div_ceil(256) * 256;
        let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("0065 live HUD readback"),
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
            timeout: Some(Duration::from_secs(30)),
        })?;
        rx.recv_timeout(Duration::from_secs(30))??;
        let mapped = buffer.slice(..).get_mapped_range();
        let bytes = mapped
            .chunks(stride as usize)
            .flat_map(|row| row[..w as usize * 4].iter().copied())
            .collect();
        drop(mapped);
        buffer.unmap();
        Ok(bytes)
    }

    struct Capture {
        host: u64,
        name: String,
        frame: bool,
    }

    /// Public runtime snapshot at one host step: UI owner declarations, accepted
    /// layout, the drawn text input and the consumer's accepted resources.
    fn snapshot(game: &Game, host: u64, generation: u64) -> Result<Value> {
        let world = game.world().map_err(|e| e.to_string())?;
        let owner = game.host().map_err(|e| e.to_string())?;
        let ui = owner.ui().ok_or("this package has no UI owner")?;
        let widgets: BTreeMap<_, _> = ui
            .widgets()
            .iter()
            .map(|w| {
                (
                    ulid(w.id),
                    json!({"parent": w.parent.map(ulid), "order": w.node.order,
                        "container": w.node.kind.is_container(),
                        "text": w.node.kind.text().map(|t| t.content.as_str())}),
                )
            })
            .collect();
        let layout = ui.layout().ok_or("UI layout")?;
        let boxes: BTreeMap<_, _> = layout
            .nodes
            .iter()
            .map(|n| {
                (
                    ulid(n.id),
                    json!({"origin": n.bounds.origin, "size": n.bounds.size,
                        "clip_origin": n.clip.origin, "clip_size": n.clip.size}),
                )
            })
            .collect();
        let input = owner.ui_input().ok_or("UI input")?;
        let drawn: Vec<_> = input
            .widgets
            .iter()
            .map(|w| {
                json!({"id": ulid(w.id), "role": format!("{:?}", w.role), "bounds": w.bounds,
                    "visible": w.visible, "clip": w.clip, "origin": w.origin,
                    "source": w.layout.source(), "size": w.layout.size(),
                    "missing_clusters": w.layout.missing_clusters().len(),
                    "glyphs": w.layout.glyphs().len(),
                    "lines": w.layout.lines().iter().map(|l| json!({"top": l.top,
                        "height": l.height, "width": l.width, "baseline": l.baseline,
                        "rtl": l.rtl})).collect::<Vec<_>>()})
            })
            .collect();
        let stats = owner.consumer().ui_stats().ok_or("UI stats")?;
        let scene = owner.consumer().stats().ok_or("scene stats")?;
        Ok(
            json!({"host_step": host, "generation": generation, "simulation_tick": world.tick(),
            "viewport": input.viewport, "widgets": widgets, "layout": boxes, "drawn": drawn,
            "ui": {"widgets": stats.widgets, "glyph_quads": stats.glyph_quads, "quads": stats.quads,
                "missing_glyphs": stats.missing_glyphs, "gpu_bytes": stats.gpu_bytes},
            "scene": {"model_entities": scene.model_entities,
                "primitive_instances": scene.primitive_instances,
                "model_triangles": scene.model_triangles, "model_draw_calls": scene.model_draw_calls,
                "diagnostic_entities": scene.diagnostic_entities},
            "accepted_frames": owner.consumer().accepted_frames()}),
        )
    }
    /// Serialized exactly as the ordinary player's report `logs` field.
    fn logs(game: &Game) -> Result<Value> {
        Ok(serde_json::to_value(
            game.logs().map_err(|e| e.to_string())?,
        )?)
    }

    pub fn main() -> Result<()> {
        let args: Vec<_> = std::env::args().skip(1).collect();
        let [data, recording, plan, out] = &args[..] else {
            return Err(
                "usage: zz_0065_live_hud <package-data> <recording.json> <plan.json> <new-output>"
                    .into(),
            );
        };
        let (data, out) = (Path::new(data), Path::new(out));
        let plan_bytes = read(Path::new(plan), 64 << 10)?;
        let plan: Value = serde_json::from_slice(&plan_bytes)?;
        let viewport: [f32; 2] = serde_json::from_value(plan["viewport"].clone())?;
        let scale = plan["scale"].as_f64().ok_or("plan scale")? as f32;
        let ticks = plan["ticks"].as_u64().ok_or("plan ticks")?;
        let max_frames = plan["max_frames"].as_u64().ok_or("plan max_frames")? as usize;
        let size = viewport.map(|v| (v * scale).round() as u32);
        let mut captures = Vec::new();
        for row in plan["captures"].as_array().ok_or("plan captures")? {
            let name = row["name"].as_str().ok_or("capture name")?;
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            {
                return Err("capture names are lowercase ASCII, digits and dashes".into());
            }
            captures.push(Capture {
                host: row["host_step"].as_u64().ok_or("capture host_step")?,
                name: name.into(),
                frame: row["frame"].as_bool().ok_or("capture frame")?,
            });
        }
        let frames = captures.iter().filter(|c| c.frame).count();
        if viewport.iter().any(|v| !(16.0..=4096.0).contains(v))
            || !(1.0..=4.0).contains(&scale)
            || size[0] > MAX_SIZE[0]
            || size[1] > MAX_SIZE[1]
            || !(1..=3600).contains(&ticks)
            || captures.is_empty()
            || captures.len() > 64
            || captures.windows(2).any(|w| w[0].host >= w[1].host)
            || captures.iter().any(|c| c.host == 0 || c.host > ticks)
            || frames > max_frames
            || max_frames > MAX_FRAMES
            || captures
                .iter()
                .enumerate()
                .any(|(i, c)| captures[..i].iter().any(|d| d.name == c.name))
        {
            return Err("plan outside packet bounds (size <= 960x540, <= 24 frames, ordered unique host steps <= ticks <= 3600)".into());
        }
        let manifest: Value = serde_json::from_slice(&read(&data.join("package.json"), 8192)?)?;
        let camera = manifest["camera"].as_str().ok_or("package camera")?;
        let mut id = [0u8; 16];
        for (i, byte) in id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(camera.get(i * 2..i * 2 + 2).ok_or("camera hex")?, 16)?;
        }
        let scene_bytes = read(&data.join("world.scene"), 128 << 20)?;
        let scene = CookedScene::from_bytes(&scene_bytes)?;
        let actions: InputActions =
            serde_json::from_slice(&read(&data.join("input-actions.json"), 256 << 10)?)?;
        let recording_bytes = read(
            Path::new(recording),
            incant_input::MAX_RECORDING_BYTES as u64,
        )?;
        let replay =
            InputReplay::from_text(std::str::from_utf8(&recording_bytes)?, 0, scene.tick_rate())?;
        if replay.has_ui() || ticks > replay.end_tick() {
            return Err("recording must cover every host step and carry no UI receipts".into());
        }
        let program = embedded::load()?;
        let metadata = program.metadata();
        let program_identity = json!({"engine": metadata.engine.to_string(),
            "target": metadata.target.to_string(), "build": metadata.build_id.to_string()});
        let seed = GameSeed::new(scene, program, actions, clips(data)?)?;
        let renderer = Arc::new(Renderer::headless().map_err(|e| e.to_string())?);
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("0065 live HUD target"),
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
        // Each generation gets fresh host settings, as the player's restart does.
        let settings = || NativeRenderSettings {
            ui: Some(UiSettings::new(viewport)),
            text: Some(incant_render_native::TextSettings::new(data.join("fonts"))),
            model_cache: data.join("models"),
            texture_cache: data.join("textures"),
            camera: StableId(id),
            consumer: GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets: Targets(texture.clone()),
            },
        };
        let started = Instant::now();
        let mut game =
            Game::from_seed(&seed, SAMPLE_RATE, settings()).map_err(|e| e.to_string())?;
        std::fs::create_dir(out)?;
        let mut fence = Fence::default();
        let mut pcm_hash = Sha256::new();
        let mut pcm_frames = 0u64;
        let mut generation = 1u64;
        let mut boundaries = Vec::new();
        let mut generations = Vec::new();
        let mut rows = Vec::new();
        let mut next = 0usize;
        let mut written = 0usize;
        for host in 1..=ticks {
            let count = game.next_output_frames().map_err(|e| e.to_string())?;
            let mut pcm = vec![0f32; count * 2];
            game.tick_with_ui(fence.filter(replay.events_at(host)?)?, &[], &mut pcm)
                .map_err(|e| e.to_string())?;
            for sample in &pcm {
                pcm_hash.update(sample.to_le_bytes());
            }
            pcm_frames += count as u64;
            while next < captures.len() && captures[next].host == host {
                let capture = &captures[next];
                let begun = Instant::now();
                let mut row = snapshot(&game, host, generation)?;
                row["name"] = capture.name.clone().into();
                if capture.frame {
                    if written == max_frames {
                        return Err("frame budget exhausted".into());
                    }
                    let rgba = pixels(&renderer, &texture)?;
                    let file = format!("{:02}-{}.rgba", written + 1, capture.name);
                    std::fs::write(out.join(&file), &rgba)?;
                    written += 1;
                    row["rgba"] = file.into();
                    row["rgba_sha256"] = hex(&Sha256::digest(&rgba)).into();
                    row["size"] = json!(size);
                }
                row["capture_seconds"] = begun.elapsed().as_secs_f64().into();
                rows.push(row);
                next += 1;
            }
            if game.take_session_request().map_err(|e| e.to_string())?
                == Some(SessionRequest::Restart)
            {
                let tick = game.world().map_err(|e| e.to_string())?.tick();
                generations.push(logs(&game)?);
                boundaries.push(
                    json!({"host_step": host, "completed_generation": generation,
                    "simulation_tick": tick, "output_frames": pcm_frames}),
                );
                game.restart_from(&seed, settings())
                    .map_err(|e| e.to_string())?;
                fence.replace_session();
                generation += 1;
            }
        }
        generations.push(logs(&game)?);
        let adapter = renderer.adapter.get_info();
        let report = json!({"format": "incant-0065-live-hud-frames", "version": 1,
            "adapter": adapter.name, "backend": format!("{:?}", adapter.backend),
            "driver": adapter.driver, "driver_info": adapter.driver_info,
            "device_type": format!("{:?}", adapter.device_type),
            "target_format": format!("{FORMAT:?}"), "viewport": viewport, "scale": scale,
            "size": size, "host_steps": ticks, "final_generation": generation,
            "final_simulation_tick": game.world().map_err(|e| e.to_string())?.tick(),
            "boundaries": boundaries, "generation_logs": generations,
            "pcm": {"sample_rate": SAMPLE_RATE, "frames": pcm_frames,
                "sha256": hex(&pcm_hash.finalize())},
            "program": program_identity,
            "inputs": {"plan_sha256": hex(&Sha256::digest(&plan_bytes)),
                "recording_sha256": hex(&Sha256::digest(&recording_bytes)),
                "world_scene_sha256": hex(&Sha256::digest(&scene_bytes))},
            "frames_written": written, "captures": rows,
            "elapsed_seconds": started.elapsed().as_secs_f64(),
            "scope": "offscreen GPU frames of the build-embedded program on accepted routes; no window, device audio, presentation or performance claim"});
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
    eprintln!("zz_0065_live_hud requires --features project,render-gpu");
    std::process::exit(2);
}
