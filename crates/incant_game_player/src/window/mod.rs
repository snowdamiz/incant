//! Optional desktop window host. Existing cooked data, embedded script, native
//! game session and retained GPU consumer remain the sole simulation path.
mod clock;
mod input;
mod probe;
mod surface;
use crate::{bounded, embedded, session, startup};
use incant_audio_runtime::{AudioManagerSettings, DeviceBackend};
use incant_game_runtime::HostedGameSession;
use incant_render_game::{NativeRenderHost, NativeRenderSettings};
use incant_render_gpu_host::{GpuRenderConsumer, GpuRenderSettings};
use incant_runtime::{CookedScene, StableId};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use surface::{SurfaceState, SurfaceTargets};
use tao::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};
type Game = HostedGameSession<DeviceBackend, NativeRenderHost<GpuRenderConsumer<SurfaceTargets>>>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
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
    #[error("this native window host supports keyboard and three-button mouse bindings only")]
    InputCapability,
    #[error("native pointer position must be finite and within one million pixels")]
    InputRange,
    #[error("native input event capacity exceeded")]
    InputCapacity,
}
struct Options {
    scene: String,
    cache: String,
    clips: String,
    actions: String,
    render: NativeRenderSettings,
    stop_after: Option<u64>,
    probe: bool,
}
impl Options {
    fn parse(args: &[String]) -> session::Result<Self> {
        let probe = args.last().is_some_and(|arg| arg == "--lifecycle-probe");
        let args = if probe { &args[..args.len() - 1] } else { args };
        if args.len() != 7 && !(args.len() == 9 && args[7] == "--stop-after") {
            return Err("expected --window SCENE AUDIO_CACHE CLIPS ACTIONS MODEL_CACHE TEXTURE_CACHE CAMERA_HEX [--stop-after TICKS]".into());
        }
        let text = &args[6];
        if text.len() != 32 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("CAMERA_HEX must contain exactly32 hexadecimal digits".into());
        }
        let mut id = [0; 16];
        for (i, v) in id.iter_mut().enumerate() {
            *v = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)?;
        }
        let stop_after = if args.len() == 9 {
            let n = args[8].parse()?;
            if !(1..=36000).contains(&n) {
                return Err("window tick bound1..36000".into());
            }
            Some(n)
        } else {
            None
        };
        Ok(Self {
            scene: args[0].clone(),
            cache: args[1].clone(),
            clips: args[2].clone(),
            actions: args[3].clone(),
            render: NativeRenderSettings {
                model_cache: args[4].clone().into(),
                texture_cache: args[5].clone().into(),
                camera: StableId(id),
                consumer: (),
            },
            stop_after: if probe { Some(40) } else { stop_after },
            probe,
        })
    }
}
pub fn run(args: &[String]) -> session::Result<()> {
    let options = Options::parse(args)?;
    let scene = CookedScene::from_bytes(&bounded(&options.scene, 128 * 1024 * 1024)?)?;
    let rate = scene.tick_rate();
    let actions = serde_json::from_slice(&bounded(&options.actions, 256 * 1024)?)?;
    input::validate_actions(&actions)?;
    let clips = startup::clips(&options.cache, &options.clips)?;
    let script = embedded::load()?;
    let mut event_loop = EventLoop::new();
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Incant native game")
            .with_inner_size(PhysicalSize::new(640, 480))
            .with_min_inner_size(PhysicalSize::new(16, 16))
            .with_max_inner_size(PhysicalSize::new(1920, 1080))
            .with_maximizable(false)
            .build(&event_loop)?,
    );
    let (mut surface, renderer, targets) = SurfaceState::new(Arc::clone(&window))?;
    let mut game = Game::new(
        scene,
        script,
        actions,
        clips,
        AudioManagerSettings::default(),
        NativeRenderSettings {
            model_cache: options.render.model_cache,
            texture_cache: options.render.texture_cache,
            camera: options.render.camera,
            consumer: GpuRenderSettings {
                renderer: Arc::clone(&renderer),
                targets,
            },
        },
    )?;
    // Startup may begin unfocused or occluded. Suspend while waiting for the
    // first frame; a device callback may precede this asynchronous command.
    game.poll_device()?;
    game.set_suspended(true)?;
    let mut suspended = true;
    let mut focused = window.is_focused();
    let mut platform_suspended = false;
    let mut input = input::PendingInput::new();
    input.reset_focus(focused);
    let mut clock = clock::Clock::new(rate, Instant::now());
    let mut retry = Instant::now();
    let mut redraw_pending = true;
    let mut presentation_refreshes = 0u64;
    let mut failure: Option<Box<dyn std::error::Error>> = None;
    let mut exit = false;
    let mut last_audio_poll = Instant::now();
    let mut probe = options.probe.then(probe::Probe::new);
    if options.probe {
        window.set_focus();
    }
    let code = event_loop.run_return(|event, _, flow| {
        let result = (|| -> session::Result<()> {
            match event {
                Event::WindowEvent {
                    window_id, event, ..
                } if window_id == window.id() => match event {
                    WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                        exit = true;
                    }
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                        surface.invalidate();
                        redraw_pending = true;
                    }
                    WindowEvent::Focused(value) => {
                        if !value && !suspended {
                            game.set_suspended(true)?;
                            suspended = true;
                            clock.reset(Instant::now());
                        }
                        focused = value;
                        input.reset_focus(value);
                    }
                    WindowEvent::Suspended => {
                        platform_suspended = true;
                        surface.invalidate();
                    }
                    WindowEvent::Resumed => {
                        platform_suspended = false;
                        surface.invalidate();
                        redraw_pending = true;
                    }
                    ref event if focused && !suspended => input.window(event)?,
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
                    if !active {
                        if !suspended {
                            game.set_suspended(true)?;
                            suspended = true;
                            input.reset_focus(focused);
                        }
                        clock.reset(now);
                        // Background resize/redraw presents committed state once;
                        // it cannot unpause input, the VM, physics or audio.
                        if clock::claim_retry(&mut retry, now)
                            && redraw_pending
                            && presentable
                            && surface.reserve(&renderer)?
                        {
                            let result = game.refresh_host();
                            let consumed = surface.consumed();
                            surface.cancel();
                            result?;
                            if !consumed {
                                return Err(Error::Reservation.into());
                            }
                            redraw_pending = false;
                            presentation_refreshes += 1;
                        }
                    } else if now >= retry {
                        let due = if suspended { 1 } else { clock.due(now) };
                        for _ in 0..due {
                            if !surface.reserve(&renderer)? {
                                if !suspended {
                                    game.set_suspended(true)?;
                                    suspended = true;
                                    input.reset_focus(focused);
                                }
                                clock.reset(now);
                                retry = now + Duration::from_millis(50);
                                break;
                            }
                            game.poll_device()?;
                            if suspended {
                                game.set_suspended(false)?;
                                suspended = false;
                                clock.reset(now);
                            }
                            let result = game.tick(input.events());
                            let consumed = surface.consumed();
                            surface.cancel(); // also releases reservation on retryable input failure
                            result?;
                            if !consumed {
                                return Err(Error::Reservation.into());
                            }
                            input.clear();
                            redraw_pending = false;
                            if options
                                .stop_after
                                .is_some_and(|n| game.world().is_ok_and(|w| w.tick() >= n))
                            {
                                exit = true;
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = result {
            failure = Some(error);
            exit = true;
        }
        *flow = if exit {
            ControlFlow::Exit
        } else {
            ControlFlow::WaitUntil(if suspended {
                retry.max(Instant::now() + Duration::from_millis(1))
            } else {
                clock.deadline()
            })
        };
    });
    surface.cancel();
    if let Some(error) = failure {
        drop(game);
        return Err(error);
    }
    if code != 0 {
        return Err(format!("native event loop failed with code{code}").into());
    }
    if let Some(probe) = &probe {
        probe.validate()?;
    }
    let report = serde_json::json!({"mode":"native-window","tick":game.world()?.tick(),"accepted_frames":game.host()?.consumer().accepted_frames(),"presentation_refreshes":presentation_refreshes,"adapter":renderer.adapter_name,"backend":format!("{:?}",renderer.adapter.get_info().backend),"on_screen":true,"dropped_wall_ticks":clock.dropped.to_string(),"surface_reconfigurations":surface.reconfigurations,"surface_recreations":surface.recreations,"surface_skipped":surface.skipped,"lifecycle_probe":probe.as_ref().map(|p|p.report())});
    drop(game); // stop device audio before returning/closing the native surface
    println!("{report}");
    Ok(())
}
