//! Optional desktop window host. Existing cooked data, embedded script, native
//! game session and retained GPU consumer remain the sole simulation path.
#[cfg(feature = "profile")]
mod cadence;
pub(crate) mod clock;
#[cfg(test)]
mod compilation_options_tests;
#[cfg(all(feature = "native-compilation", target_os = "macos"))]
mod compilations;
mod devices;
#[cfg(all(feature = "native-display-link", target_os = "macos"))]
mod display_link;
mod gamepad;
mod haptics;
mod ime;
mod input;
#[cfg(test)]
mod pacing_tests;
#[cfg(all(feature = "native-presentations", target_os = "macos"))]
mod presentations;
mod probe;
#[cfg(all(feature = "native-presentations", target_os = "macos"))]
use presentations::{Boundary, Role, Status};
mod replay;
mod state_log;
mod surface;
mod termination;
mod ui_input;
use crate::{bounded, embedded, session, startup};
use incant_audio_runtime::{AudioManagerSettings, DeviceBackend};
use incant_game_runtime::{GameSeed, HostedGameSession};
use incant_render_game::{NativeRenderHost, NativeRenderSettings, RenderInputConsumer, UiSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings};
use incant_runtime::{CookedScene, StableId};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use surface::{SurfaceState, SurfaceTargets};
use tao::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};
type Game = HostedGameSession<DeviceBackend, NativeRenderHost<GpuRenderConsumer<SurfaceTargets>>>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Pacing(#[from] crate::pacing::Error),
    #[cfg(all(feature = "native-minimum-duration", target_os = "macos"))]
    #[error(transparent)]
    NativePacing(#[from] wgpu_hal::metal::minimum_duration::Error),
    #[error("minimum-duration60 requires the Metal backend")]
    PacingBackend,
    #[error("minimum-duration60 requires FIFO with maximum frame latency 2")]
    PacingConfiguration,
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    #[error("native presentation observation failed: {0}")]
    Presentation(String),
    #[error("native rendering failed: {0}")]
    Render(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("native surface creation failed: {0}")]
    Surface(#[from] incant_render::wgpu::CreateSurfaceError),
    #[error("native surface lacks required sRGB format/alpha capabilities")]
    SurfaceCapabilities,
    #[error("native surface validation failed")]
    SurfaceValidation,
    #[error("native surface reservation invariant failed")]
    Reservation,
    #[error(
        "this native window host supports keyboard, three-button mouse and gamepad IDs0..15 only"
    )]
    InputCapability,
    #[error("native gamepad backend initialization failed")]
    GamepadBackend,
    #[error("native gamepad haptic backend rejected the request")]
    HapticBackend,
    #[error("native gamepad haptic generation exhausted")]
    HapticGeneration,
    #[error(transparent)]
    HapticRequest(#[from] incant_input::haptics::HapticError),
    #[error(transparent)]
    GamepadInput(#[from] incant_input::InputError),
    #[error("native pointer position must be finite and within one million pixels")]
    InputRange,
    #[error("native input scale factor must be a finite positive normal value")]
    InputScale,
    #[error("native input event capacity exceeded")]
    InputCapacity,
    #[error("native UI receipt count or text byte capacity exceeded")]
    UiCapacity,
    #[error("native IME session counter exhausted")]
    ImeSession,
    #[error("native IME operation refused: {0}")]
    Ime(#[from] tao::text_input::TextInputError),
    #[error("native window rendering has no screen-space UI consumer")]
    UiUnsupported,
}
struct Options {
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    compilation: Option<(String, usize)>,
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    capture: Option<(String, usize)>,
    scene: String,
    cache: String,
    clips: String,
    actions: String,
    render: NativeRenderSettings,
    stop_after: Option<u64>,
    recording: Option<String>,
    observations: Option<String>,
    state_log: Option<String>,
    probe: bool,
}
impl Options {
    fn parse(args: &[String]) -> session::Result<Self> {
        const USAGE: &str = "expected --window SCENE AUDIO_CACHE CLIPS ACTIONS MODEL_CACHE TEXTURE_CACHE CAMERA_HEX [--font-cache FONT_CACHE] [--stop-after TICKS] [--input-recording FILE] [--observe PLAN] [--window-state-log FILE]";
        let probe = args.last().is_some_and(|arg| arg == "--lifecycle-probe");
        let args = if probe { &args[..args.len() - 1] } else { args };
        if args.len() < 7 {
            return Err(USAGE.into());
        }
        let (args, mut rest) = args.split_at(7);
        // Explicit checked font catalog for the composed text binding; never
        // inferred from another cache or the system.
        let mut text = None;
        if let [flag, path, tail @ ..] = rest
            && flag == "--font-cache"
        {
            text = Some(incant_render_game::TextSettings::new(path.into()));
            rest = tail;
        }
        let mut stop_after = None;
        let mut recording = None;
        let mut observations = None;
        let mut state_log = None;
        let mut compilation_output = None;
        let mut compilation_capacity = None;
        let mut presentation_output = None;
        let mut presentation_capacity = None;
        while let [flag, value, tail @ ..] = rest {
            match flag.as_str() {
                "--stop-after" if stop_after.is_none() => {
                    let n = value.parse()?;
                    if !(1..=36000).contains(&n) {
                        return Err("window tick bound1..36000".into());
                    }
                    stop_after = Some(n);
                }
                "--input-recording" if recording.is_none() => recording = Some(value.clone()),
                "--observe" if observations.is_none() => observations = Some(value.clone()),
                "--window-state-log" if state_log.is_none() => state_log = Some(value.clone()),
                "--compilation-output" if compilation_output.is_none() => {
                    compilation_output = Some(value.clone())
                }
                "--compilation-capacity" if compilation_capacity.is_none() => {
                    compilation_capacity = Some(value.parse::<usize>()?)
                }
                "--presentation-output" if presentation_output.is_none() => {
                    presentation_output = Some(value.clone())
                }
                "--presentation-capacity" if presentation_capacity.is_none() => {
                    presentation_capacity = Some(value.parse::<usize>()?)
                }
                _ => return Err(USAGE.into()),
            }
            rest = tail;
        }
        if !rest.is_empty() {
            return Err(USAGE.into());
        }
        if (recording.is_some() || observations.is_some()) && (probe || stop_after.is_none()) {
            return Err(
                "recording/observation requires --stop-after and conflicts with lifecycle probe"
                    .into(),
            );
        }
        if let Some(path) = &state_log {
            state_log::validate(path, stop_after, probe)?;
        }
        if presentation_output.is_some() || presentation_capacity.is_some() {
            if !cfg!(all(feature = "native-presentations", target_os = "macos")) {
                return Err("presentation capture requires native-presentations on macOS".into());
            }
            if probe
                || stop_after.is_none()
                || presentation_output.as_ref().is_none_or(|p| {
                    p.is_empty() || p.len() > 4096 || p.chars().any(char::is_control)
                })
                || presentation_capacity.is_none_or(|n| !(1..=36_000).contains(&n))
            {
                return Err("presentation capture needs bounded --stop-after, --presentation-output and --presentation-capacity1..36000; no lifecycle probe".into());
            }
        }
        if compilation_output.is_some() || compilation_capacity.is_some() {
            if !cfg!(all(feature = "native-compilation", target_os = "macos")) {
                return Err("compilation capture requires native-compilation on macOS".into());
            }
            if probe
                || stop_after.is_none()
                || compilation_output.as_ref().is_none_or(|p| {
                    p.is_empty() || p.len() > 4096 || p.chars().any(char::is_control)
                })
                || compilation_capacity.is_none_or(|n| !(1..=4096).contains(&n))
            {
                return Err("compilation capture needs bounded --stop-after, --compilation-output and --compilation-capacity1..4096; no lifecycle probe".into());
            }
        }
        let hex = &args[6];
        if hex.len() != 32 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("CAMERA_HEX must contain exactly32 hexadecimal digits".into());
        }
        let mut id = [0; 16];
        for (i, v) in id.iter_mut().enumerate() {
            *v = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)?;
        }
        Ok(Self {
            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
            compilation: compilation_output.zip(compilation_capacity),
            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
            capture: presentation_output.zip(presentation_capacity),
            scene: args[0].clone(),
            cache: args[1].clone(),
            clips: args[2].clone(),
            actions: args[3].clone(),
            render: NativeRenderSettings {
                ui: None,
                text,
                model_cache: args[4].clone().into(),
                texture_cache: args[5].clone().into(),
                camera: StableId(id),
                consumer: (),
            },
            stop_after: if probe { Some(40) } else { stop_after },
            recording,
            observations,
            state_log,
            probe,
        })
    }
}
pub fn run(
    args: &[String],
    pacing: crate::pacing::Mode,
    wake_mode: crate::wake::Mode,
    batch_mode: crate::batching::Mode,
) -> session::Result<serde_json::Value> {
    let options = Options::parse(args)?;
    let batch_latest = batch_mode.selected();
    // Explicit selection refuses wrong scene/OS before creating diagnostic
    // outputs, a window, audio device or GPU. Ordinary startup order is retained.
    let admitted_scene = if pacing.selected() || wake_mode.selected() {
        let scene = CookedScene::from_bytes(&bounded(&options.scene, 128 * 1024 * 1024)?)?;
        pacing.admit_scene(scene.tick_rate())?;
        pacing.admit_platform()?;
        wake_mode.admit_scene(scene.tick_rate())?;
        if wake_mode.selected() {
            #[cfg(all(feature = "native-display-link", target_os = "macos"))]
            if !display_link::supported() {
                return Err(crate::wake::Error::Unsupported.into());
            }
            #[cfg(not(all(feature = "native-display-link", target_os = "macos")))]
            return Err(crate::wake::Error::Unsupported.into());
        }
        Some(scene)
    } else {
        None
    };
    let mut state_log = options
        .state_log
        .as_deref()
        .map(|path| state_log::Log::open(path, options.stop_after, Instant::now()))
        .transpose()?;
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    let capture = options
        .capture
        .as_ref()
        .map(|(path, capacity)| {
            presentations::Capture::new(
                path,
                *capacity,
                options
                    .stop_after
                    .ok_or("presentation capture lacks host bound")?,
            )
            .map(|mut capture| {
                capture.enable_batches(batch_latest);
                capture
            })
        })
        .transpose()?;
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    let mut compilation = options
        .compilation
        .as_ref()
        .map(|(path, capacity)| {
            compilations::Capture::new(
                path,
                *capacity,
                options
                    .stop_after
                    .ok_or("compilation capture lacks host bound")?,
            )
        })
        .transpose()?;
    let scene = match admitted_scene {
        Some(scene) => scene,
        None => CookedScene::from_bytes(&bounded(&options.scene, 128 * 1024 * 1024)?)?,
    };
    let has_ui = scene.has_ui();
    // Refuse before opening a window, audio backend or GPU. The rendering lane
    // opts in only when its real canonical screen-space consumer is implemented.
    if has_ui && !GpuRenderConsumer::<SurfaceTargets>::SUPPORTS_UI {
        return Err(Error::UiUnsupported.into());
    }
    let rate = scene.tick_rate();
    let replay = options
        .recording
        .as_deref()
        .map(|path| -> session::Result<_> {
            Ok(replay::Replay::parse(
                std::str::from_utf8(&bounded(path, incant_input::MAX_RECORDING_BYTES)?)?,
                rate,
                options.stop_after,
            )?)
        })
        .transpose()?;
    let mut observations = options
        .observations
        .as_deref()
        .map(|path| -> session::Result<_> {
            Ok(crate::observe::Plan::parse(
                &bounded(path, crate::observe::MAX_PLAN_BYTES)?,
                options
                    .stop_after
                    .ok_or("observation requires host step bound")?,
            )?)
        })
        .transpose()?;
    let actions = serde_json::from_slice(&bounded(&options.actions, 256 * 1024)?)?;
    input::validate_actions(&actions)?;
    let clips = startup::clips(&options.cache, &options.clips)?;
    let script = embedded::load()?;
    let mut event_loop = EventLoop::new();
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Incant native game")
            .with_inner_size(LogicalSize::new(640, 480))
            .with_min_inner_size(PhysicalSize::new(16, 16))
            .with_max_inner_size(PhysicalSize::new(1920, 1080))
            .with_maximizable(false)
            .build(&event_loop)?,
    );
    let initial_size = window.inner_size();
    let ui_settings = has_ui
        .then(|| {
            logical_viewport(
                [initial_size.width, initial_size.height],
                window.scale_factor(),
            )
        })
        .transpose()?
        .map(UiSettings::new);
    let mut gamepads = devices::Devices::new(replay.is_none())?;
    let (mut surface, renderer, targets) = SurfaceState::new(
        Arc::clone(&window),
        pacing,
        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
        compilation.as_mut().map(|c| &mut c.renderer),
    )?;
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    macro_rules! compilation_scope {
        ($phase:ident, $generation:expr, $step:expr) => {
            compilations::enter(
                &compilation,
                incant_render::compilation::Phase::$phase,
                $generation,
                surface
                    .recreations
                    .checked_add(1)
                    .ok_or("compilation surface identity exhausted")?,
                $step,
            )?
        };
    }
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    if let Some(capture) = capture {
        surface.install_capture(capture)?;
    }
    let seed = GameSeed::new(scene, script, actions, clips)?;
    let render_settings = |targets, ui| NativeRenderSettings {
        ui,
        text: options.render.text.clone(),
        model_cache: options.render.model_cache.clone(),
        texture_cache: options.render.texture_cache.clone(),
        camera: options.render.camera,
        consumer: GpuRenderSettings {
            renderer: Arc::clone(&renderer),
            targets,
        },
    };
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    let mut startup_compilation = compilation_scope!(Startup, 1, 0);
    let mut game = Game::from_seed_suspended(
        &seed,
        AudioManagerSettings::default(),
        render_settings(targets, ui_settings),
    )?;
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    {
        compilations::commit(&mut startup_compilation);
        drop(startup_compilation);
    }
    game.poll_device()?;
    if let Some(plan) = &mut observations {
        plan.observe_at(0, game.world()?, game.input()?)?;
    }
    let mut initial_frame = true;
    let mut generation = 1u64;
    let mut host_steps = 0u64;
    let mut gameplay_presentations = 0u64;
    let mut controls = crate::restart::InputFence::default();
    let mut suspended = true;
    let mut focused = window.is_focused();
    let mut platform_suspended = false;
    let mut input = input::PendingInput::new();
    input.reset_focus(focused);
    let mut ui_input = ui_input::UiInput::new();
    ui_input.defer_surface();
    ui_input.synchronize(game.host()?.ui(), &mut input)?;
    if has_ui && let Some((serial, enabled)) = ui_input.text_session(game.host()?.ui(), false)? {
        window
            .set_text_input_session(serial, enabled)
            .map_err(Error::Ime)?;
    }
    let mut ui_resize = false;
    let mut clock = clock::Clock::new(rate, Instant::now());
    let mut retry = Instant::now();
    let mut redraw_pending = true;
    let mut presentation_refreshes = 0u64;
    let mut ui_relayouts = 0u64;
    let mut termination = termination::Termination::default();
    let mut last_audio_poll = Instant::now();
    let mut probe = options.probe.then(probe::Probe::new);
    #[cfg(feature = "profile")]
    let mut cadence = cadence::Cadence::new();
    // No native query, world read, serialization or file I/O is added when
    // this explicitly enabled diagnostic is absent. A snapshot is observational.
    macro_rules! observe_state {
        ($reason:expr) => {
            if let Some(log) = &mut state_log {
                let observed_now = Instant::now();
                let size = window.inner_size();
                log.observe(
                    observed_now,
                    $reason,
                    state_log::Snapshot {
                        host_steps,
                        world_tick: game.world()?.tick(),
                        generation,
                        cached_focused: focused,
                        native_focused: window.is_focused(),
                        platform_suspended,
                        simulation_suspended: suspended,
                        minimized: window.is_minimized(),
                        supported_size: surface.has_supported_size(),
                        physical_size: [size.width, size.height],
                        retry_in_us: u64::try_from(
                            retry.saturating_duration_since(observed_now).as_micros(),
                        )
                        .unwrap_or(u64::MAX),
                        surface_skipped: surface.skipped,
                        surface_reconfigurations: surface.reconfigurations,
                        surface_recreations: surface.recreations,
                    },
                )?;
            }
        };
    }
    observe_state!(state_log::Reason::Initial);
    if options.probe {
        window.set_focus();
    }
    #[cfg(all(feature = "native-display-link", target_os = "macos"))]
    let mut display_wake = wake_mode
        .selected()
        .then(|| display_link::Host::new(Arc::clone(&window), event_loop.create_proxy()))
        .transpose()?;
    let code = event_loop.run_return(|event, _, flow| {
        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
        if let Event::NewEvents(cause) = &event {
            surface.presentation_new_events(*cause);
        }
        termination.event(|exit| {
            match event {
                #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                Event::UserEvent(()) => {
                    if let Some(wake) = &mut display_wake {
                        wake.acknowledge();
                    }
                }
                Event::WindowEvent {
                    window_id, event, ..
                } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                        *exit = true;
                    }
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                        if let Some(wake) = &mut display_wake {
                            wake.boundary(Instant::now());
                        }
                        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                        if let Some(c) = &compilation {
                            c.boundary(compilations::Boundary::Resize);
                        }
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        surface.presentation_boundary(Boundary::Resize);
                        #[cfg(feature = "profile")]
                        cadence.interrupt();
                        if has_ui {
                            ui_input.layout_pending(game.host()?.ui(), &mut input)?;
                            ui_resize = true;
                        }
                        surface.invalidate();
                        redraw_pending = true;
                        observe_state!(state_log::Reason::Resize);
                    }
                    WindowEvent::Focused(value) => {
                        #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                        if let Some(wake) = &mut display_wake {
                            wake.boundary(Instant::now());
                        }
                        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                        if let Some(c) = &compilation {
                            c.boundary(compilations::Boundary::Focus);
                        }
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        surface.presentation_boundary(Boundary::Focus);
                        #[cfg(feature = "profile")]
                        if !value {
                            cadence.interrupt();
                        }
                        if !value && !suspended {
                            gamepads.pause_haptics()?;
                            game.set_suspended(true)?;
                            suspended = true;
                            clock.reset(Instant::now());
                        }
                        focused = value;
                        input.reset_focus(value);
                        ui_input.reset_focus();
                        gamepads.discard_pending();
                        observe_state!(if value {
                            state_log::Reason::FocusGained
                        } else {
                            state_log::Reason::FocusLost
                        });
                    }
                    WindowEvent::Suspended => {
                        #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                        if let Some(wake) = &mut display_wake {
                            wake.boundary(Instant::now());
                        }
                        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                        if let Some(c) = &compilation {
                            c.boundary(compilations::Boundary::Suspend);
                        }
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        surface.presentation_boundary(Boundary::Suspend);
                        #[cfg(feature = "profile")]
                        cadence.interrupt();
                        gamepads.pause_haptics()?;
                        platform_suspended = true;
                        if has_ui {
                            input.reset_focus(false);
                            ui_input.reset_focus();
                            gamepads.discard_pending();
                        }
                        surface.invalidate();
                        observe_state!(state_log::Reason::Suspended);
                    }
                    WindowEvent::Resumed => {
                        #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                        if let Some(wake) = &mut display_wake {
                            wake.boundary(Instant::now());
                        }
                        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                        if let Some(c) = &compilation {
                            c.boundary(compilations::Boundary::Resume);
                        }
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        surface.presentation_boundary(Boundary::Resume);
                        platform_suspended = false;
                        surface.invalidate();
                        redraw_pending = true;
                        observe_state!(state_log::Reason::Resumed);
                    }
                    ref event
                        if replay.is_none() && ui_input.accepts_delivery(focused, suspended) =>
                    {
                        // InputEvent uses logical pixels; query this window's current scale per event.
                        if has_ui {
                            ui_input.window(
                                game.host()?.ui(),
                                &mut input,
                                event,
                                window.scale_factor(),
                            )?;
                        } else {
                            input.window(event, window.scale_factor())?;
                        }
                    }
                    _ => {}
                },
                // Raw DeviceEvent::MouseMotion is deliberately not forwarded:
                // InputRuntime already derives displacement from CursorMoved.
                // A future captured-pointer mode must choose the raw stream.
                Event::RedrawRequested(id) if id == window.id() => {
                    redraw_pending = true;
                }
                Event::MainEventsCleared => {
                    let now = Instant::now();
                    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                    surface.presentation_pulse(now);
                    observe_state!(state_log::Reason::Pulse);
                    if let Some(probe) = &mut probe {
                        probe.observe(&window, game.world()?.tick(), &mut surface, &renderer)?;
                    }
                    if now.duration_since(last_audio_poll) >= Duration::from_millis(100) {
                        game.poll_audio()?;
                        game.poll_device()?;
                        renderer.check_device().map_err(Error::Render)?;
                        last_audio_poll = now;
                    }
                    let presentable = !platform_suspended
                        && !window.is_minimized()
                        && surface.has_supported_size();
                    let active = focused && presentable;
                    #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                    let wake_ready = match display_wake.as_mut() {
                        Some(wake) => wake.poll(now, active)?,
                        None => true,
                    };
                    #[cfg(not(all(feature = "native-display-link", target_os = "macos")))]
                    let wake_ready = true;
                    let mut batch_refresh = false;
                    if has_ui && ui_resize && presentable {
                        #[cfg(feature = "profile")]
                        cadence.interrupt();
                        if now < retry {
                            return Ok(());
                        }
                        if !{
                            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                            surface.presentation_attempt(Role::UiResize, generation, host_steps);
                            reserve_observed(
                                &mut surface,
                                &renderer,
                                &mut state_log,
                                #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                                &compilation,
                            )?
                        } {
                            if !suspended {
                                gamepads.pause_haptics()?;
                                game.set_suspended(true)?;
                                suspended = true;
                                ui_input.defer_surface();
                            }
                            clock.reset(now);
                            retry = now + Duration::from_millis(50);
                            return Ok(());
                        }
                        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                        let mut refresh_compilation =
                            compilation_scope!(Background, generation, host_steps);
                        // Resize and actual consumption share this callback and
                        // reservation, even when no simulation tick is due.
                        let result = crate::profile::presentation(|| -> session::Result<()> {
                            ui_relayouts += fit_ui(
                                &mut game,
                                &surface,
                                window.scale_factor(),
                                &mut ui_input,
                                &mut input,
                            )?;
                            game.refresh_host()?;
                            Ok(())
                        });
                        let consumed = surface.consumed();
                        surface.cancel();
                        result?;
                        if !consumed {
                            return Err(Error::Reservation.into());
                        }
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        surface.presentation_complete(Status::RefreshCompleted);
                        ui_input.presented(game.host()?.ui(), &mut input)?;
                        #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                        compilations::commit(&mut refresh_compilation);
                        ui_resize = false;
                        redraw_pending = false;
                        presentation_refreshes += 1;
                        if batch_latest {
                            // This opportunity already displayed committed state.
                            // Keep the wall-clock debt for the next opportunity;
                            // still process inactivity and text-session lifecycle.
                            initial_frame = false;
                            batch_refresh = true;
                        }
                    }
                    // Actual inactivity retains only connection state. A temporary
                    // UI surface wait preserves releases alongside queued presses.
                    gamepads.poll(&mut input, ui_input.accepts_delivery(active, suspended))?;
                    if !active {
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        if !suspended {
                            surface.presentation_boundary(Boundary::Inactive);
                        }
                        #[cfg(feature = "profile")]
                        cadence.interrupt();
                        if ui_input.awaiting_surface() {
                            input.reset_focus(focused);
                            ui_input.reset_focus();
                            gamepads.discard_pending();
                        }
                        if !suspended {
                            gamepads.pause_haptics()?;
                            game.set_suspended(true)?;
                            suspended = true;
                            input.reset_focus(focused);
                            ui_input.reset_focus();
                            gamepads.discard_pending();
                        }
                        clock.reset(now);
                        // Background resize/redraw presents committed state once;
                        // it cannot unpause input, the VM, physics or audio.
                        if clock::claim_retry(&mut retry, now) && redraw_pending && presentable && {
                            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                            surface.presentation_attempt(
                                if initial_frame {
                                    Role::Initial
                                } else {
                                    Role::Background
                                },
                                generation,
                                host_steps,
                            );
                            reserve_observed(
                                &mut surface,
                                &renderer,
                                &mut state_log,
                                #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                                &compilation,
                            )?
                        } {
                            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                            let mut refresh_compilation =
                                compilation_scope!(Background, generation, host_steps);
                            let result = crate::profile::presentation(|| -> session::Result<()> {
                                if has_ui {
                                    ui_relayouts += fit_ui(
                                        &mut game,
                                        &surface,
                                        window.scale_factor(),
                                        &mut ui_input,
                                        &mut input,
                                    )?;
                                }
                                game.refresh_host()?;
                                Ok(())
                            });
                            let consumed = surface.consumed();
                            surface.cancel();
                            result?;
                            if !consumed {
                                return Err(Error::Reservation.into());
                            }
                            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                            surface.presentation_complete(Status::RefreshCompleted);
                            ui_input.presented(game.host()?.ui(), &mut input)?;
                            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                            compilations::commit(&mut refresh_compilation);
                            redraw_pending = false;
                            presentation_refreshes += 1;
                            initial_frame = false;
                        }
                    } else if now >= retry && wake_ready && !batch_refresh {
                        if initial_frame {
                            #[cfg(feature = "profile")]
                            cadence.interrupt();
                            if !{
                                #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                                surface.presentation_attempt(Role::Initial, generation, host_steps);
                                reserve_observed(
                                    &mut surface,
                                    &renderer,
                                    &mut state_log,
                                    #[cfg(all(
                                        feature = "native-compilation",
                                        target_os = "macos"
                                    ))]
                                    &compilation,
                                )?
                            } {
                                retry = now + Duration::from_millis(50);
                                clock.reset(now);
                                return Ok(());
                            }
                            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                            let mut refresh_compilation =
                                compilation_scope!(Background, generation, host_steps);
                            let result = crate::profile::presentation(|| game.refresh_host());
                            let consumed = surface.consumed();
                            surface.cancel();
                            result?;
                            if !consumed {
                                return Err(Error::Reservation.into());
                            }
                            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                            surface.presentation_complete(Status::RefreshCompleted);
                            ui_input.presented(game.host()?.ui(), &mut input)?;
                            presentation_refreshes += 1;
                            redraw_pending = false;
                            initial_frame = false;
                            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                            compilations::commit(&mut refresh_compilation);
                            clock.reset(now);
                            return Ok(());
                        }
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        let schedule_deadline = (!suspended).then(|| clock.deadline());
                        #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                        let dropped_before = clock.dropped;
                        let due = if suspended { 1 } else { clock.due(now) };
                        for _tick_index in 0..due {
                            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                            let mut tick_compilation = compilation_scope!(
                                Gameplay,
                                generation,
                                host_steps
                                    .checked_add(1)
                                    .ok_or("compilation host step exhausted")?
                            );
                            // Includes surface acquisition, input/UI preparation and
                            // normal haptics. This is CPU host time, not scanout.
                            #[cfg(feature = "profile")]
                            let host_started = cadence.begin();
                            if (!batch_latest || _tick_index == 0)
                                && !{
                                    #[cfg(all(
                                        feature = "native-presentations",
                                        target_os = "macos"
                                    ))]
                                    {
                                        surface.presentation_gameplay_attempt(
                                            generation,
                                            host_steps + 1,
                                            presentations::Schedule {
                                                pulse: now,
                                                deadline: schedule_deadline,
                                                due,
                                                ordinal: _tick_index,
                                                discarded_ticks: clock.dropped - dropped_before,
                                            },
                                        );
                                        if batch_latest {
                                            surface.presentation_batch_begin(due);
                                        }
                                    }
                                    reserve_observed(
                                        &mut surface,
                                        &renderer,
                                        &mut state_log,
                                        #[cfg(all(
                                            feature = "native-compilation",
                                            target_os = "macos"
                                        ))]
                                        &compilation,
                                    )?
                                }
                            {
                                #[cfg(feature = "profile")]
                                cadence.interrupt();
                                if !suspended {
                                    gamepads.pause_haptics()?;
                                    game.set_suspended(true)?;
                                    suspended = true;
                                    if has_ui {
                                        ui_input.defer_surface();
                                    } else {
                                        input.reset_focus(focused);
                                        gamepads.discard_pending();
                                    }
                                }
                                clock.reset(now);
                                retry = now + Duration::from_millis(50);
                                break;
                            }
                            game.poll_device()?;
                            if has_ui {
                                ui_relayouts += fit_ui(
                                    &mut game,
                                    &surface,
                                    window.scale_factor(),
                                    &mut ui_input,
                                    &mut input,
                                )?;
                            }
                            if suspended {
                                game.set_suspended(false)?;
                                suspended = false;
                                clock.reset(now);
                                ui_input.resume(&mut input, focused);
                                gamepads.synchronize(&mut input)?;
                            }
                            if game.supports_haptics()? {
                                game.set_haptic_capabilities(gamepads.haptic_capabilities())?;
                            }
                            // Read by accepted host step, not simulation tick or
                            // redraw count. No native input mixes into a recording.
                            let packet = match &replay {
                                Some(replay) => replay.events(host_steps)?,
                                None => input.events(),
                            };
                            let events = controls.filter(packet)?;
                            // Preserve all fixed-step input/audio/resource admissions.
                            // Only the final accepted state consumes the reservation.
                            let defer = batch_latest
                                && _tick_index + 1 < due
                                && !options.stop_after.is_some_and(|n| host_steps + 1 >= n);
                            surface.admission_only(defer)?;
                            crate::profile::frame(|| -> session::Result<()> {
                                let result = if has_ui {
                                    game.tick_with_ui(
                                        events,
                                        if replay.is_some() {
                                            &[]
                                        } else {
                                            ui_input.receipts()
                                        },
                                    )
                                } else {
                                    game.tick(events)
                                };
                                let consumed = surface.consumed();
                                if result.is_err() || !defer {
                                    surface.cancel(); // release on retryable input failure too
                                }
                                result?;
                                if consumed == defer {
                                    return Err(Error::Reservation.into());
                                }
                                Ok(())
                            })?;
                            gamepads.commit();
                            input.clear();
                            ui_input.accepted();
                            if !batch_latest {
                                ui_input.presented(game.host()?.ui(), &mut input)?;
                                redraw_pending = false;
                            }
                            host_steps = host_steps.checked_add(1).ok_or("host step exhausted")?;
                            #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                            {
                                compilations::commit(&mut tick_compilation);
                                drop(tick_compilation);
                            }
                            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                            if !batch_latest {
                                surface.presentation_complete(Status::HostCommitted);
                            }
                            if let Some(plan) = &mut observations {
                                plan.observe_at(host_steps, game.world()?, game.input()?)?;
                            }
                            let restart = game.take_session_request()?.is_some();
                            // A restart can shorten a batch after admission. Display
                            // the accepted old generation before destroying its owner.
                            if defer && restart {
                                surface.admission_only(false)?;
                                let result = crate::profile::presentation(|| game.refresh_host());
                                let consumed = surface.consumed();
                                surface.cancel();
                                result?;
                                if !consumed {
                                    return Err(Error::Reservation.into());
                                }
                            }
                            let displayed = !defer || restart;
                            if displayed {
                                gameplay_presentations = gameplay_presentations
                                    .checked_add(1)
                                    .ok_or("gameplay presentation count exhausted")?;
                                if batch_latest {
                                    ui_input.presented(game.host()?.ui(), &mut input)?;
                                    redraw_pending = false;
                                }
                            }
                            #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                            {
                                if batch_latest {
                                    surface.presentation_batch_accept(host_steps, displayed);
                                    if displayed {
                                        surface.presentation_complete(Status::HostCommitted);
                                    }
                                }
                            }
                            let haptics = game.take_haptics()?;
                            if restart {
                                gamepads.pause_haptics()?;
                                #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                                if displayed {
                                    surface.presentation_host_finished();
                                }
                                #[cfg(feature = "profile")]
                                {
                                    cadence.finish(host_started);
                                    cadence.interrupt();
                                }
                                let _profile =
                                    incant_profile::scope(incant_profile::Phase::Restart);
                                let next = generation
                                    .checked_add(1)
                                    .ok_or("session generation exhausted")?;
                                // Full ownership replacement, never component edits or a save load.
                                game.stop();
                                let retired = ui_input.replace_session()?;
                                if has_ui && let Some((serial, enabled)) = retired {
                                    window
                                        .set_text_input_session(serial, enabled)
                                        .map_err(Error::Ime)?;
                                }
                                controls.replace_session();
                                input.reset_focus(focused);
                                let size = window.inner_size();
                                let ui = has_ui
                                    .then(|| {
                                        logical_viewport(
                                            [size.width, size.height],
                                            window.scale_factor(),
                                        )
                                    })
                                    .transpose()?
                                    .map(UiSettings::new);
                                #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                                if let Some(c) = &compilation {
                                    c.boundary(compilations::Boundary::Restart);
                                }
                                #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                                let mut restart_compilation =
                                    compilation_scope!(Startup, next, host_steps);
                                game = Game::from_seed_suspended(
                                    &seed,
                                    AudioManagerSettings::default(),
                                    render_settings(surface.replacement_targets()?, ui),
                                )?;
                                #[cfg(all(feature = "native-compilation", target_os = "macos"))]
                                {
                                    compilations::commit(&mut restart_compilation);
                                    drop(restart_compilation);
                                }
                                generation = next;
                                #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                                if let Some(wake) = &mut display_wake {
                                    wake.boundary(Instant::now());
                                }
                                #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                                surface.presentation_boundary(Boundary::Restart);
                                suspended = true;
                                initial_frame = true;
                                redraw_pending = true;
                                ui_resize = false;
                                clock.reset(now);
                            }
                            if !restart {
                                gamepads.haptics(&haptics)?;
                                #[cfg(all(feature = "native-presentations", target_os = "macos"))]
                                if displayed {
                                    surface.presentation_host_finished();
                                }
                                #[cfg(feature = "profile")]
                                cadence.finish(host_started);
                            }
                            if options.stop_after.is_some_and(|n| host_steps >= n) {
                                *exit = true;
                            }
                            if restart || *exit {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
            if has_ui && !*exit {
                let active = replay.is_none()
                    && focused
                    && !platform_suspended
                    && !window.is_minimized()
                    && surface.has_supported_size()
                    && (!suspended || ui_input.awaiting_surface());
                if let Some((serial, enabled)) = ui_input.text_session(game.host()?.ui(), active)? {
                    window
                        .set_text_input_session(serial, enabled)
                        .map_err(Error::Ime)?;
                }
            }
            Ok(())
        });
        *flow = if termination.requested() {
            // Cleanup is attempted even after an earlier composed failure.
            let _ = gamepads.pause_haptics();
            ControlFlow::Exit
        } else {
            ControlFlow::WaitUntil(if suspended {
                retry.max(Instant::now() + Duration::from_millis(1))
            } else {
                // Health deadline only; it cannot authorize gameplay.
                #[cfg(all(feature = "native-display-link", target_os = "macos"))]
                let deadline = display_wake
                    .as_ref()
                    .map_or_else(|| clock.deadline(), display_link::Host::deadline);
                #[cfg(not(all(feature = "native-display-link", target_os = "macos")))]
                let deadline = clock.deadline();
                deadline
            })
        };
    });
    #[cfg(all(feature = "native-display-link", target_os = "macos"))]
    if let Some(wake) = &mut display_wake {
        wake.stop();
    }
    surface.cancel();
    let result = (|| -> session::Result<serde_json::Value> {
        termination.finish(code, || {
            gamepads.pause_haptics()?;
            if has_ui && let Some((serial, enabled)) = ui_input.text_session(None, false)? {
                window
                    .set_text_input_session(serial, enabled)
                    .map_err(Error::Ime)?;
            }
            Ok(())
        })?;
        if let Some(probe) = &probe {
            probe.validate()?;
        }
        let mut report = serde_json::json!({"mode":"native-window","tick":game.world()?.tick(),"accepted_frames":game.host()?.consumer().accepted_frames(),"presentation_refreshes":presentation_refreshes,"adapter":renderer.adapter_name,"backend":format!("{:?}",renderer.adapter.get_info().backend),"on_screen":true,"dropped_wall_ticks":clock.dropped.to_string(),"surface_reconfigurations":surface.reconfigurations,"surface_recreations":surface.recreations,"surface_skipped":surface.skipped,"discarded_stale_text_events":ui_input.stale_text_events(),"lifecycle_probe":probe.as_ref().map(|p|p.report())});
        if pacing.selected() {
            report["native_pacing"] = serde_json::json!({
                "mode":"metal-minimum-duration60",
                "minimum_previous_display_seconds":1.0 / 60.0,
                "simulation_tick_rate":rate,
                "diagnostic_only":true,
                "performance_gate_passed":false
            });
        }
        if batch_latest {
            report["native_batch"] = serde_json::json!({
                "mode":"latest", "simulation_steps":host_steps,
                "gameplay_presentations":gameplay_presentations,
                "intermediate_admissions":host_steps - gameplay_presentations,
                "maximum_ticks_per_batch":clock::MAX_CATCH_UP,
                "diagnostic_only":true,"performance_gate_passed":false
            });
        }
        #[cfg(all(feature = "native-display-link", target_os = "macos"))]
        if let Some(wake) = &display_wake {
            report["native_wake"] = wake.report();
        }
        #[cfg(feature = "profile")]
        cadence.append(&mut report);
        report["host_steps"] = host_steps.into();
        report["audio_device"] = serde_json::json!({
            "scope":"current-session-owner",
            "diagnostics":game.device_audio_diagnostics()?,
            "glitch_free_output_verified":false
        });
        if let Some(replay) = &replay {
            report["input_recording"] = serde_json::json!({"sha256":replay.sha256,
            "requested_host_steps":options.stop_after,"accepted_host_steps":host_steps,
            "completed":options.stop_after == Some(host_steps),"source":"recorded-physical-input",
            "physical_devices":false,"native_focus_and_surface_required":true});
        }
        if let Some(plan) = observations {
            report["observations"] = serde_json::to_value(plan.finish(game.world()?)?)?;
        }
        if generation > 1 {
            report["session"] = serde_json::json!({"generation":generation,"host_tick":host_steps,
                "simulation_tick":game.world()?.tick(),"accepted_frames":gameplay_presentations+presentation_refreshes});
        }
        if let Some(ui) = game.host()?.ui_input() {
            let stats = game.host()?.consumer().ui_stats();
            let uploads = renderer.ui_uploads();
            report["ui"] = serde_json::json!({
                "viewport":ui.viewport,"scale_factor":window.scale_factor(),
                "widgets":ui.widgets.len(),"relayouts":ui_relayouts,
                "focus":game.host()?.ui().and_then(|ui|ui.focus()).map(|f|ulid::Ulid::from_bytes(f.widget().0).to_string()),
                "glyph_quads":stats.map(|s|s.glyph_quads),"quads":stats.map(|s|s.quads),"gpu_bytes":stats.map(|s|s.gpu_bytes),
                "uploads":{"atlas_textures":uploads.atlas_textures,"atlas_bytes":uploads.atlas_bytes,"quad_buffers":uploads.quad_buffers,"quad_bytes":uploads.quad_bytes,"frame_writes":uploads.frame_writes}
            });
        }
        Ok(report)
    })();
    drop(game); // stop audio before diagnostic I/O and the bounded callback drain
    let logged = state_log
        .as_mut()
        .map(|log| log.finish(Instant::now(), host_steps, result.is_ok()))
        .transpose()
        .map(|_| ());
    let result = state_log::preserve_result(result, logged);
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    let result = {
        let observed = compilation
            .as_mut()
            .map(|c| c.finish(host_steps, generation, code, &result))
            .transpose()
            .map(|_| ());
        compilations::preserve_result(result, observed)
    };
    #[cfg(all(feature = "native-presentations", target_os = "macos"))]
    {
        let observed = surface.finish_capture(host_steps, generation, code, &result);
        // A sidecar error never replaces the original gameplay/window failure.
        presentations::preserve_result(result, observed)
    }
    #[cfg(not(all(feature = "native-presentations", target_os = "macos")))]
    result
}

// Retain the exact existing reservation result and ordering; the optional log
// records only its outcome. It cannot reserve, retry or admit a frame itself.
fn reserve_observed(
    surface: &mut SurfaceState,
    renderer: &incant_render::Renderer,
    log: &mut Option<state_log::Log>,
    #[cfg(all(feature = "native-compilation", target_os = "macos"))] compilation: &Option<
        compilations::Capture,
    >,
) -> Result<bool, Error> {
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    let before = [
        surface.reconfigurations,
        surface.recreations,
        surface.skipped,
    ];
    let result = surface.reserve(renderer);
    #[cfg(all(feature = "native-compilation", target_os = "macos"))]
    if let Some(capture) = compilation {
        capture.reservation(
            before,
            [
                surface.reconfigurations,
                surface.recreations,
                surface.skipped,
            ],
            result.as_ref().ok().copied(),
        );
    }
    let ready = result?;
    if let Some(log) = log {
        log.reservation(ready);
    }
    Ok(ready)
}

/// Layout uses the dimensions of the reserved frame. Window dimensions can
/// change before their resize notification reaches this callback.
fn logical_viewport(size: [u32; 2], scale: f64) -> Result<[f32; 2], Error> {
    if !scale.is_normal() || scale <= 0. {
        return Err(Error::InputScale);
    }
    let logical = size.map(|n| f64::from(n) / scale);
    if logical
        .iter()
        .any(|n| !n.is_finite() || !(1. ..=65_536.).contains(n))
    {
        return Err(Error::InputRange);
    }
    Ok(logical.map(|n| n as f32))
}
/// Relayout is admitted before simulation or presentation. Pointer receipts
/// still refer to the previous displayed geometry and must be fenced; text
/// receipts keep their independent focus token. No unchanged-frame allocation.
fn fit_ui(
    game: &mut Game,
    surface: &SurfaceState,
    scale: f64,
    ui_input: &mut ui_input::UiInput,
    input: &mut input::PendingInput,
) -> session::Result<u64> {
    let wanted = logical_viewport(surface.reserved_size()?, scale)?;
    if game
        .host()?
        .ui_input()
        .is_some_and(|ui| ui.viewport == wanted)
    {
        return Ok(0);
    }
    ui_input.layout_pending(game.host()?.ui(), input)?;
    game.resize_ui(wanted)?;
    ui_input.invalidate_pointer(game.host()?.ui(), input)?;
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::Options;
    fn args(tail: &[&str]) -> Vec<String> {
        [
            "scene",
            "audio",
            "clips",
            "actions",
            "models",
            "textures",
            "0123456789abcdef0123456789abcdef",
        ]
        .iter()
        .chain(tail)
        .map(|s| (*s).to_owned())
        .collect()
    }
    #[test]
    fn state_log_is_default_off_and_requires_trusted_path_and_bounded_host() {
        assert!(Options::parse(&args(&[])).unwrap().state_log.is_none());
        let admitted = Options::parse(&args(&[
            "--window-state-log",
            "trusted.jsonl",
            "--stop-after",
            "781",
        ]))
        .unwrap();
        assert_eq!(admitted.state_log.as_deref(), Some("trusted.jsonl"));
        assert_eq!(admitted.stop_after, Some(781));
        for tail in [
            vec!["--window-state-log", "trusted.jsonl"],
            vec!["--stop-after", "1", "--window-state-log", ""],
            vec!["--stop-after", "1", "--window-state-log", "bad\npath"],
            vec!["--stop-after", "1", "--window-state-log"],
            vec![
                "--stop-after",
                "1",
                "--window-state-log",
                "a",
                "--window-state-log",
                "b",
            ],
            vec![
                "--stop-after",
                "1",
                "--window-state-log",
                "a",
                "--lifecycle-probe",
            ],
        ] {
            assert!(Options::parse(&args(&tail)).is_err(), "{tail:?}");
        }
    }
    #[test]
    fn presentation_capture_requires_explicit_host_bounds_and_platform_feature() {
        for tail in [
            vec!["--presentation-output", "capture.json"],
            vec!["--stop-after", "6", "--presentation-output", "capture.json"],
            vec!["--stop-after", "6", "--presentation-capacity", "8"],
            vec![
                "--stop-after",
                "6",
                "--presentation-output",
                "capture.json",
                "--presentation-capacity",
                "0",
            ],
            vec![
                "--stop-after",
                "6",
                "--presentation-output",
                "capture.json",
                "--presentation-capacity",
                "36001",
            ],
            vec![
                "--stop-after",
                "6",
                "--presentation-output",
                "capture.json",
                "--presentation-output",
                "other",
                "--presentation-capacity",
                "8",
            ],
        ] {
            assert!(Options::parse(&args(&tail)).is_err());
        }
        let full = args(&[
            "--stop-after",
            "6",
            "--presentation-output",
            "capture.json",
            "--presentation-capacity",
            "8",
        ]);
        assert_eq!(
            Options::parse(&full).is_ok(),
            cfg!(all(feature = "native-presentations", target_os = "macos"))
        );
    }
    #[test]
    fn physical_surface_and_pointer_coordinates_share_exact_logical_scale() {
        let position = tao::dpi::PhysicalPosition::new(641., 481.);
        for scale in [1., 1.5, 2., 3.] {
            let viewport = super::logical_viewport([641, 481], scale).unwrap();
            let pointer = super::input::logical_position(position, scale).unwrap();
            assert_eq!(viewport, pointer.map(|n| n as f32));
        }
        // Do not silently stretch a subpixel or overflowed logical viewport:
        // that would disagree with the independently scaled pointer positions.
        for (size, scale) in [([0, 480], 1.), ([16, 16], 32.), ([1920, 1080], 0.001)] {
            assert!(super::logical_viewport(size, scale).is_err());
        }
        for scale in [0., -1., f64::NAN, f64::INFINITY, f64::MIN_POSITIVE / 2.] {
            assert!(super::logical_viewport([640, 480], scale).is_err());
        }
    }
    #[test]
    fn explicit_font_cache_and_tick_bound_parse_in_fixed_order() {
        let plain = Options::parse(&args(&[])).unwrap();
        assert!(plain.render.text.is_none() && plain.stop_after.is_none());
        let text = Options::parse(&args(&["--font-cache", "fonts", "--stop-after", "3"])).unwrap();
        assert_eq!(
            text.render.text.unwrap().font_cache,
            std::path::Path::new("fonts")
        );
        assert_eq!(text.stop_after, Some(3));
        let probe = Options::parse(&args(&["--font-cache", "fonts", "--lifecycle-probe"])).unwrap();
        assert!(probe.probe && probe.render.text.is_some());
        assert_eq!(probe.stop_after, Some(40));
        for bad in [
            &["--stop-after", "3", "--font-cache", "fonts"][..],
            &["--font-cache"],
            &["--fonts", "fonts"],
            &["--stop-after", "0"],
            &["--font-cache", "a", "--font-cache", "b"],
        ] {
            assert!(Options::parse(&args(bad)).is_err(), "{bad:?}");
        }
    }
    #[test]
    fn recording_and_observation_are_explicit_bounded_non_probe_modes() {
        let parsed = Options::parse(&args(&[
            "--input-recording",
            "route.json",
            "--stop-after",
            "100",
            "--observe",
            "checks.json",
        ]))
        .unwrap();
        assert_eq!(parsed.recording.as_deref(), Some("route.json"));
        assert_eq!(parsed.observations.as_deref(), Some("checks.json"));
        assert_eq!(parsed.stop_after, Some(100));
        for tail in [
            vec!["--input-recording", "route.json"],
            vec!["--observe", "checks.json"],
            vec!["--stop-after", "10", "--input-recording"],
            vec![
                "--stop-after",
                "10",
                "--input-recording",
                "a",
                "--input-recording",
                "b",
            ],
            vec!["--stop-after", "10", "--observe", "a", "--observe", "b"],
            vec!["--stop-after", "10", "--stop-after", "20"],
            vec![
                "--stop-after",
                "10",
                "--input-recording",
                "a",
                "--lifecycle-probe",
            ],
        ] {
            assert!(Options::parse(&args(&tail)).is_err(), "{tail:?}");
        }
    }
}
