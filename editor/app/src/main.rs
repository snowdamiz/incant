use incant_cmd::{Actor, Command};
use incant_doc::{Project, schema_registry};
use incant_render::{Renderer, Viewport, wgpu};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
mod assets;
mod menu;
mod project;
mod provider;
mod source_watch;
mod window;
struct Editor {
    provider: Arc<provider::ProviderRuntime>,
    bus: Arc<Mutex<project::LoadState>>,
    viewport: Mutex<Option<Viewport>>,
    alive: AtomicBool,
    console: Mutex<Vec<ConsoleEvent>>,
    viewport_error: Mutex<Option<String>>,
    asset_root: Option<PathBuf>,
    importing: AtomicBool,
    source_diagnostics: Mutex<Vec<incant_import::WatchDiagnostic>>,
}
#[derive(Clone, Serialize)]
struct ConsoleEvent {
    id: String,
    level: &'static str,
    message: String,
}
impl ConsoleEvent {
    fn new(level: &'static str, message: String) -> Self {
        Self {
            id: incant_doc::new_id(),
            level,
            message,
        }
    }
}
#[tauri::command]
fn engine_read(state: tauri::State<'_, Arc<Editor>>) -> Result<Value, String> {
    let load = state.bus.lock().map_err(|_| "engine lock failed")?;
    let bus = match &*load {
        project::LoadState::Ready(bus) => bus,
        project::LoadState::Loading => return Ok(json!({"status":"loading"})),
        project::LoadState::Failed(error) => {
            return Ok(
                json!({"status":"error", "error":{"code":error.code(),"message":error.to_string()}}),
            );
        }
    };
    Ok(
        json!({"status":"ready","project":bus.project(),"revision":bus.revision(),"can_redo":bus.can_redo(),"applied":bus.history().len(),"history":bus.history().iter().chain(bus.redo_history()).map(|tx|json!({"id":tx.id,"description":tx.description,"actor":tx.actor})).collect::<Vec<_>>(),"schemas":schema_registry(),"console":state.console.lock().map_err(|_|"console lock failed")?.clone(),"viewport_error":state.viewport_error.lock().map_err(|_|"viewport lock failed")?.clone(),"source_diagnostics": state.source_diagnostics.lock().map_err(|_|"source diagnostics lock failed")?.clone(),"asset_import": if state.asset_root.is_some() {json!({"available":true})}else{json!({"available":false,"reason":"Open a saved project to import assets."})}}),
    )
}
#[tauri::command]
fn engine_execute(
    state: tauri::State<'_, Arc<Editor>>,
    commands: Vec<Command>,
    description: String,
    expected_revision: u64,
) -> Result<Value, String> {
    {
        let mut load = state.bus.lock().map_err(|_| "engine lock failed")?;
        let bus = load.bus_mut()?;
        bus.execute(
            commands,
            Actor::user("editor"),
            description,
            Some(expected_revision),
        )
        .map_err(|e| e.to_string())?;
    }
    engine_read(state)
}
#[tauri::command]
fn engine_history(state: tauri::State<'_, Arc<Editor>>, redo: bool) -> Result<Value, String> {
    {
        let mut load = state.bus.lock().map_err(|_| "engine lock failed")?;
        let bus = load.bus_mut()?;
        if redo { bus.redo() } else { bus.undo() }.map_err(|e| e.to_string())?;
    }
    engine_read(state)
}
#[tauri::command]
fn viewport_bounds(
    state: tauri::State<'_, Arc<Editor>>,
    rect: [f32; 4],
    corner_radii: [f32; 4],
) -> Result<(), String> {
    if rect
        .iter()
        .chain(corner_radii.iter())
        .any(|v| !v.is_finite() || *v < 0.)
    {
        return Err("invalid viewport bounds".into());
    }
    *state.viewport.lock().map_err(|_| "viewport lock failed")? = Some(Viewport {
        rect,
        corner_radii,
        // Claude's revision-1 canvas color, specified in editor/ui/TITLEBAR.md.
        canvas_srgb: [11, 12, 15],
    });
    Ok(())
}
fn main() {
    let project_path = std::env::args_os().nth(1).map(PathBuf::from);
    let asset_root = project_path.as_ref().map(|path| {
        path.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."))
            .to_path_buf()
    });
    let editor = Arc::new(Editor {
        provider: Arc::new(provider::ProviderRuntime::new()),
        bus: Arc::new(Mutex::new(project::LoadState::Loading)),
        viewport: Mutex::new(None),
        alive: AtomicBool::new(true),
        console: Mutex::new(vec![]),
        viewport_error: Mutex::new(None),
        asset_root,
        importing: AtomicBool::new(false),
        source_diagnostics: Mutex::new(vec![]),
    });
    tauri::Builder::default()
        .manage(editor.clone())
        .invoke_handler(tauri::generate_handler![
            engine_read,
            engine_execute,
            engine_history,
            assets::engine_import,
            viewport_bounds,
            window::window_read,
            window::window_action,
            provider::provider_read,
            provider::provider_action
        ])
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            menu::install(app.handle())?;
            editor
                .provider
                .start(app.handle().clone(), "restore".into(), None, false)?;
            let config = tauri::utils::config::WindowConfig {
                label: "main".into(),
                background_color: Some(tauri::utils::config::Color(11, 12, 15, 255)),
                traffic_light_position: cfg!(target_os = "macos")
                    .then_some(tauri::utils::config::LogicalPosition { x: 14., y: 22. }),
                ..Default::default()
            };
            let builder = tauri::window::WindowBuilder::from_config(app, &config)?
                .title("Incant — Phase 0")
                .inner_size(1440., 900.)
                .min_inner_size(1000., 650.);
            #[cfg(target_os = "macos")]
            let builder = builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true);
            #[cfg(not(target_os = "macos"))]
            let builder = builder.decorations(false);
            let window = builder.build()?;
            let size = window.inner_size()?;
            let webview =
                tauri::WebviewBuilder::new("editor", tauri::WebviewUrl::App("index.html".into()))
                    .initialization_script(include_str!("../../bridge/native.generated.js"))
                    .transparent(true)
                    .auto_resize();
            window.add_child(webview, tauri::PhysicalPosition::new(0, 0), size)?;
            let project_app = app.handle().clone();
            project::start(
                editor.bus.clone(),
                project::LOAD_TIMEOUT,
                move || project::open(project_path.as_deref()),
                move || {
                    use tauri::Emitter;
                    let _ = project_app.emit_to(
                        tauri::EventTarget::webview("editor"),
                        "incant:engine-changed",
                        (),
                    );
                },
            );
            // The native surface is owned by the native window, independently of the webview.
            let instance = wgpu::Instance::default();
            let surface = instance.create_surface(window.clone())?;
            let renderer = pollster::block_on(Renderer::new(&instance, Some(&surface)))
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            editor.console.lock().unwrap().push(ConsoleEvent::new(
                "info",
                format!("Native wgpu adapter: {}", renderer.adapter_name),
            ));
            let close = editor.clone();
            let observed_window = window.clone();
            let account_app = app.handle().clone();
            window.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Focused(true)) {
                    let _ =
                        close
                            .provider
                            .start(account_app.clone(), "restore".into(), None, false);
                }
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    close.alive.store(false, Ordering::Relaxed);
                } else if matches!(
                    event,
                    tauri::WindowEvent::Resized(_)
                        | tauri::WindowEvent::Focused(_)
                        | tauri::WindowEvent::ScaleFactorChanged { .. }
                ) {
                    window::publish_state(&observed_window);
                }
            });
            source_watch::start(editor.clone(), app.handle().clone());
            let shared = editor.clone();
            let render_app = app.handle().clone();
            std::thread::spawn(move || {
                let mut configured = (0, 0);
                let empty = Project::empty("Loading");
                let format = surface
                    .get_capabilities(&renderer.adapter)
                    .formats
                    .into_iter()
                    .find(|f| f.is_srgb())
                    .unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
                while shared.alive.load(Ordering::Relaxed) {
                    let Ok(size) = window.inner_size() else {
                        break;
                    };
                    if size.width == 0 || size.height == 0 {
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                    if configured != (size.width, size.height) {
                        surface.configure(
                            &renderer.device,
                            &wgpu::SurfaceConfiguration {
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                format,
                                width: size.width,
                                height: size.height,
                                present_mode: wgpu::PresentMode::Fifo,
                                desired_maximum_frame_latency: 2,
                                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                                view_formats: vec![],
                            },
                        );
                        configured = (size.width, size.height);
                    }
                    let frame = match surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(frame)
                        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                        _ => {
                            configured = (0, 0);
                            std::thread::sleep(Duration::from_millis(50));
                            continue;
                        }
                    };
                    let rect = shared.viewport.lock().ok().and_then(|r| *r).filter(|v| {
                        let r = v.rect;
                        r[2] > 0.
                            && r[3] > 0.
                            && r[0] + r[2] <= size.width as f32
                            && r[1] + r[3] <= size.height as f32
                    });
                    let project = match shared.bus.lock() {
                        Ok(load) => match &*load {
                            project::LoadState::Ready(bus) => bus.project().clone(),
                            _ => empty.clone(),
                        },
                        Err(_) => break,
                    };
                    match renderer.draw(
                        &project,
                        &frame.texture.create_view(&Default::default()),
                        format,
                        size.width,
                        size.height,
                        rect,
                    ) {
                        Ok(buffer) => {
                            renderer.queue.submit([buffer]);
                            frame.present();
                        }
                        Err(error) => {
                            if let Ok(mut status) = shared.viewport_error.lock() {
                                *status = Some(error.to_string());
                            }
                            if let Ok(mut log) = shared.console.lock() {
                                log.push(ConsoleEvent::new(
                                    "error",
                                    format!("Render error: {error}"),
                                ));
                            }
                            use tauri::Emitter;
                            let _ = render_app.emit_to(
                                tauri::EventTarget::webview("editor"),
                                "incant:engine-changed",
                                (),
                            );
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(16));
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Incant native editor failed");
}
