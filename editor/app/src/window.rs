use serde::{Deserialize, Serialize};
use tauri::{Emitter, Window};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowChrome {
    platform: &'static str,
    controls: &'static str,
    maximized: bool,
    fullscreen: bool,
    focused: bool,
    leading_inset: u32,
    trailing_inset: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowAction {
    Drag,
    Minimize,
    Maximize,
    Close,
}

#[tauri::command]
pub fn window_read(window: Window) -> Result<WindowChrome, String> {
    let fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    Ok(WindowChrome {
        platform: std::env::consts::OS,
        controls: if cfg!(target_os = "macos") {
            "native-overlay"
        } else {
            "custom"
        },
        maximized: window.is_maximized().map_err(|e| e.to_string())?,
        fullscreen,
        focused: window.is_focused().map_err(|e| e.to_string())?,
        leading_inset: if cfg!(target_os = "macos") && !fullscreen {
            78
        } else {
            0
        },
        trailing_inset: 0,
    })
}

#[tauri::command]
pub fn window_action(window: Window, action: WindowAction) -> Result<(), String> {
    match action {
        WindowAction::Drag => window.start_dragging(),
        WindowAction::Minimize => window.minimize(),
        WindowAction::Maximize => {
            if window.is_maximized().map_err(|e| e.to_string())? {
                window.unmaximize()
            } else {
                window.maximize()
            }
        }
        WindowAction::Close => window.close(),
    }
    .map_err(|e| e.to_string())
}

pub fn publish_state(window: &Window) {
    if let Ok(chrome) = window_read(window.clone()) {
        let _ = window.emit("incant:window-changed", chrome);
    }
}
