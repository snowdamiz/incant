use super::{Action, Message, Request, Service, clock::Clock};
use incant_audio_runtime::{AudioManagerSettings, DeviceBackend};
use incant_input::{InputEvent, MAX_EVENTS_PER_TICK, MouseButton};
use incant_play::{PlayMode, PlaySession};
use incant_render::{Renderer, RetainedTarget, Viewport, wgpu};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings, RenderFrame, TargetProvider};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, mpsc},
    time::Instant,
};

type Game = PlaySession<DeviceBackend, GpuRenderConsumer<Targets>>;
pub struct Worker {
    service: Arc<Service>,
    rx: mpsc::Receiver<Message>,
    renderer: Arc<Renderer>,
    game: Option<Game>,
    slot: Rc<RefCell<Option<Frame>>>,
    running: bool,
    input: Vec<InputEvent>,
    step: Option<mpsc::Sender<Result<Value, String>>>,
    clock: Clock,
    held: bool,
}
#[derive(Debug, thiserror::Error)]
#[error("Native Play frame reservation is unavailable")]
struct SurfaceError;
struct Targets(Rc<RefCell<Option<Frame>>>);
struct Frame {
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
    viewport: Viewport,
}
impl TargetProvider for Targets {
    type Error = SurfaceError;
    type Frame = Frame;
    fn acquire(&mut self, _: &Renderer) -> Result<Frame, SurfaceError> {
        self.0.borrow_mut().take().ok_or(SurfaceError)
    }
}
impl RenderFrame for Frame {
    type Error = SurfaceError;
    fn target(&self) -> RetainedTarget<'_> {
        RetainedTarget {
            view: &self.view,
            format: self.format,
            width: self.width,
            height: self.height,
            viewport: Some(self.viewport),
        }
    }
    fn present(self) -> Result<(), SurfaceError> {
        self.texture.present();
        Ok(())
    }
}
impl Worker {
    pub fn new(
        service: Arc<Service>,
        rx: mpsc::Receiver<Message>,
        renderer: Arc<Renderer>,
    ) -> Self {
        Self {
            service,
            rx,
            renderer,
            game: None,
            slot: Rc::new(RefCell::new(None)),
            running: false,
            input: Vec::with_capacity(MAX_EVENTS_PER_TICK),
            step: None,
            clock: Clock::new(Instant::now()),
            held: true,
        }
    }
    pub fn active(&self) -> bool {
        self.game.is_some()
    }
    fn reset_clock(&mut self) {
        self.clock.reset(Instant::now());
    }
    fn status(&self) -> Result<Value, String> {
        serde_json::to_value(self.service.snapshot()?).map_err(|e| e.to_string())
    }
    pub fn drain(&mut self) {
        for _ in 0..16 {
            let Ok(message) = self.rx.try_recv() else {
                break;
            };
            if self.step.is_some()
                && !matches!(&message.request, Request::Action(Action::Stop { .. }))
            {
                let _ = message
                    .reply
                    .send(Err("Wait for the pending step to finish".into()));
                continue;
            }
            if let Request::Action(Action::Step { generation }) = &message.request {
                let valid = self
                    .service
                    .snapshot()
                    .is_ok_and(|s| s.generation == *generation)
                    && self
                        .game
                        .as_ref()
                        .is_some_and(|g| g.mode() == PlayMode::Paused)
                    && !self.running;
                if valid && self.step.is_none() {
                    self.step = Some(message.reply);
                    continue;
                }
                let _ = message.reply.send(Err(
                    "Pause Play before stepping; only one step may be pending".into(),
                ));
                continue;
            }
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.apply(message.request)
            }))
            .unwrap_or_else(|_| {
                self.fail("Native Play operation unwound; start a new session".into());
                Err("Native Play operation unwound".into())
            });
            let _ = message.reply.send(result);
        }
    }
    fn apply(&mut self, request: Request) -> Result<Value, String> {
        let current = self.service.snapshot()?.generation;
        match request {
            Request::Load {
                generation,
                revision,
                prepared,
            } => {
                if generation != current {
                    return Err("Play startup was cancelled".into());
                }
                let scene = prepared.scene.clone();
                let game = Game::new(
                    *prepared,
                    AudioManagerSettings::default(),
                    GpuRenderSettings {
                        renderer: self.renderer.clone(),
                        targets: Targets(self.slot.clone()),
                    },
                )
                .map_err(|e| e.to_string())?;
                let spatial = game.is_spatial();
                self.game = Some(game);
                self.running = true;
                self.held = true;
                release_input(&mut self.input);
                self.clock = Clock::new(Instant::now());
                self.service.edit(|s| {
                    s.phase = "running";
                    s.tick = 0;
                    s.discarded_ticks = 0;
                    s.presented_frames = 0;
                    s.scene = Some(scene);
                    s.source_revision = Some(revision);
                    s.spatial = spatial;
                    s.suspended = true;
                    s.error = None;
                    Ok(())
                })?;
            }
            Request::Action(action) => {
                if action.generation() != current {
                    return Err("Play session changed; refresh and retry".into());
                }
                if matches!(action, Action::Stop { .. }) {
                    let next = current.checked_add(1).ok_or("Play generation exhausted")?;
                    self.game = None;
                    self.slot.borrow_mut().take();
                    self.input.clear();
                    self.running = false;
                    self.reset_clock();
                    if let Some(reply) = self.step.take() {
                        let _ = reply.send(Err("Play stopped before the requested step".into()));
                    }
                    self.service.edit(|s| {
                        s.generation = next;
                        s.phase = "stopped";
                        s.tick = 0;
                        s.discarded_ticks = 0;
                        s.presented_frames = 0;
                        s.scene = None;
                        s.source_revision = None;
                        s.spatial = false;
                        s.suspended = false;
                        s.error = None;
                        Ok(())
                    })?;
                    return self.status();
                }
                let game = self.game.as_mut().ok_or("Start Play first")?;
                match action {
                    Action::Pause { .. } => {
                        if game.mode() != PlayMode::Paused {
                            game.pause().map_err(|e| e.to_string())?;
                        }
                        self.running = false;
                        self.held = true;
                        release_input(&mut self.input);
                        self.reset_clock();
                    }
                    Action::Resume { .. } => {
                        self.running = true;
                        self.reset_clock();
                    }
                    Action::Input { events, .. } => {
                        if !self.running || self.held {
                            return Err("Game input is inactive".into());
                        }
                        validate_events(&events)?;
                        if self.input.len() + events.len() > MAX_EVENTS_PER_TICK {
                            return Err("Game input queue is full".into());
                        }
                        self.input.extend(events);
                    }
                    Action::Inspect { entity, .. } => {
                        let id = entity
                            .parse::<ulid::Ulid>()
                            .map_err(|_| "Invalid entity ID")?;
                        if id.to_string() != entity {
                            return Err("Entity ID must be canonical".into());
                        }
                        let world = game.world().map_err(|e| e.to_string())?;
                        let value = world
                            .inspect(incant_runtime::StableId(id.to_bytes()))
                            .ok_or("Entity is absent from this runtime world")?;
                        return Ok(
                            json!({"generation":current,"tick":world.tick(),"entity":entity,
                            "parent":value.parent.map(|id|ulid::Ulid::from_bytes(id.0).to_string()),
                            "transform":value.transform,"velocity":value.velocity,"worldTransform":value.world_transform}),
                        );
                    }
                    Action::Start { .. } | Action::Step { .. } | Action::Stop { .. } => {
                        unreachable!("routed separately")
                    }
                }
                self.publish()?;
            }
        }
        self.status()
    }
    fn publish(&self) -> Result<(), String> {
        let Some(game) = &self.game else {
            return Ok(());
        };
        let tick = game.world().map_err(|e| e.to_string())?.tick();
        let frames = game
            .consumer()
            .map_err(|e| e.to_string())?
            .map_or(0, |consumer| consumer.accepted_frames());
        self.service.edit(|s| {
            s.phase = if self.running { "running" } else { "paused" };
            s.tick = tick;
            s.discarded_ticks = self.clock.discarded;
            s.presented_frames = frames;
            s.suspended = self.held;
            Ok(())
        })
    }
    pub fn unavailable(&mut self) {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.hold())).is_err() {
            self.fail("Native Play operation unwound; start a new session".into());
        }
    }
    fn hold(&mut self) {
        if let Some(reply) = self.step.take() {
            let _ = reply.send(Err(
                "The viewport is unavailable; retry the step when visible".into(),
            ));
        }
        if let Some(game) = &mut self.game {
            let result = game.poll_device().and_then(|_| {
                if game.mode() != PlayMode::Paused {
                    game.pause()
                } else {
                    Ok(())
                }
            });
            if let Err(error) = result {
                self.fail(error.to_string());
                return;
            }
            if !self.held {
                release_input(&mut self.input);
            }
            self.held = true;
            self.reset_clock();
            let _ = self.publish();
        }
    }
    pub fn fail(&mut self, error: String) {
        self.game = None;
        self.slot.borrow_mut().take();
        self.input.clear();
        self.running = false;
        if let Some(reply) = self.step.take() {
            let _ = reply.send(Err(error.clone()));
        }
        if let Ok(status) = self.service.snapshot() {
            self.service.failed(status.generation, error);
        }
    }
    /// Own one real surface frame. A native accepted tick must submit through the
    /// retained GPU consumer; paused redraws use its existing committed world.
    pub fn frame(
        &mut self,
        texture: wgpu::SurfaceTexture,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        viewport: Viewport,
        focused: bool,
    ) {
        let now = Instant::now();
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
                let game = self.game.as_mut().ok_or("Play stopped")?;
                game.poll_device().map_err(|e| e.to_string())?;
                if !focused && self.step.is_none() {
                    if game.mode() != PlayMode::Paused {
                        game.pause().map_err(|e| e.to_string())?;
                    }
                    self.held = true;
                    release_input(&mut self.input);
                    self.clock.reset(now);
                } else if self.held {
                    release_input(&mut self.input);
                    self.held = false;
                    self.clock.reset(now);
                }
                let rate = game.world().map_err(|e| e.to_string())?.tick_rate();
                let stepping = self.step.is_some();
                let advancing = if self.running && !self.held {
                    if game.mode() != PlayMode::Running {
                        game.resume().map_err(|e| e.to_string())?;
                    }
                    self.clock.due(now, rate)
                } else {
                    self.clock.reset(now);
                    false
                };
                let view = texture.texture.create_view(&Default::default());
                let frame = Frame {
                    texture,
                    view,
                    format,
                    width,
                    height,
                    viewport,
                };
                if game.is_spatial() {
                    *self.slot.borrow_mut() = Some(frame);
                } else {
                    clear(&self.renderer, frame)?;
                }
                let result = if stepping {
                    game.step(&self.input)
                } else if advancing {
                    game.advance(&self.input)
                } else {
                    game.refresh().map(|_| ())
                };
                if stepping || advancing {
                    self.input.clear();
                }
                result.map_err(|e| e.to_string())?;
                if self.slot.borrow().is_some() {
                    return Err("Native Play did not consume its surface reservation".into());
                }
                self.publish()?;
                Ok(())
            }))
            .unwrap_or_else(|_| Err("Native Play operation unwound; start a new session".into()));
        match result {
            Ok(()) => {
                if let Some(reply) = self.step.take() {
                    let _ = reply.send(self.status());
                }
            }
            Err(error) => self.fail(error),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if !self.service.snapshot().is_ok_and(|s| s.phase == "error") {
            self.fail("Native Play worker stopped".into());
        }
    }
}
fn clear(renderer: &Renderer, frame: Frame) -> Result<(), String> {
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    let channel = |v: u8| {
        let v = f64::from(v) / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let color = frame.viewport.canvas_srgb;
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Native nonvisual Play canvas"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: channel(color[0]),
                        g: channel(color[1]),
                        b: channel(color[2]),
                        a: 1.,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    renderer.queue.submit([encoder.finish()]);
    frame.texture.present();
    renderer.check_device().map_err(|e| e.to_string())
}
/// Native window focus permits simulation, but only the game view's explicit
/// input stream may grant gameplay focus. Hold/start/resume also discard stale
/// queued presses, so no frame briefly borrows another editor panel's focus.
fn release_input(input: &mut Vec<InputEvent>) {
    input.clear();
    input.push(InputEvent::Focus { focused: false });
}
fn validate_events(events: &[InputEvent]) -> Result<(), String> {
    for event in events {
        match event {
            InputEvent::Focus { .. }
            | InputEvent::Key { .. }
            | InputEvent::MouseButton {
                button: MouseButton::Left | MouseButton::Right | MouseButton::Middle,
                ..
            } => {}
            InputEvent::PointerPosition { position }
                if position
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.) => {}
            InputEvent::Wheel { delta, .. }
                if delta.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.) => {}
            _ => return Err("Unsupported or invalid game input event".into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editor_resume_releases_game_input_until_the_view_explicitly_takes_focus() {
        use incant_input::{InputRuntime, KeyCode};
        let press = |code| InputEvent::Key { code, down: true };
        let mut runtime = InputRuntime::default();
        runtime
            .advance(
                &[
                    press(KeyCode::KeyD),
                    InputEvent::MouseButton {
                        button: MouseButton::Left,
                        down: true,
                    },
                ],
                1. / 60.,
            )
            .unwrap();
        let mut queued = vec![InputEvent::Focus { focused: true }, press(KeyCode::KeyW)];
        release_input(&mut queued);
        let frame = runtime.advance(&queued, 1. / 60.).unwrap();
        assert!(!frame.focused);
        assert!(frame.keyboard.held.is_empty());
        assert!(frame.mouse.buttons.held.is_empty());
        assert!(frame.keyboard.released.contains(&KeyCode::KeyD));
        // Neither a subsequent frame nor a key packet grants the editor's focus.
        assert!(!runtime.advance(&[], 1. / 60.).unwrap().focused);
        let frame = runtime.advance(&[press(KeyCode::KeyW)], 1. / 60.).unwrap();
        assert!(!frame.focused);
        assert!(frame.keyboard.held.is_empty());
        // Only the explicit game-view stream reopens input.
        let frame = runtime
            .advance(
                &[InputEvent::Focus { focused: true }, press(KeyCode::KeyW)],
                1. / 60.,
            )
            .unwrap();
        assert!(frame.focused);
        assert!(frame.keyboard.held.contains(&KeyCode::KeyW));
    }

    #[test]
    fn unsupported_devices_and_nonfinite_pointer_values_refuse_whole_packets() {
        let valid = InputEvent::Focus { focused: true };
        for bad in [
            InputEvent::GamepadConnected { id: 0 },
            InputEvent::PointerMotion { delta: [1., 2.] },
            InputEvent::PointerPosition {
                position: [f64::NAN, 0.],
            },
            InputEvent::MouseButton {
                button: MouseButton::Back,
                down: true,
            },
        ] {
            assert!(validate_events(&[valid.clone(), bad]).is_err());
        }
        assert!(
            validate_events(&[valid, InputEvent::PointerPosition { position: [1., 2.] }]).is_ok()
        );
    }
}
