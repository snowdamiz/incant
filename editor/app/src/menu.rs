//! macOS menu history is routed by the editor according to the focused control.
#[cfg(target_os = "macos")]
pub fn install(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{Emitter, EventTarget, menu::MenuItem};

    let menu = tauri::menu::Menu::default(app)?;
    for item in menu.items()? {
        if let Some(edit) = item.as_submenu()
            && edit.text()? == "Edit"
        {
            // Tauri's predefined Undo/Redo invoke Cocoa's text undo manager and
            // consume Cmd+Z before our webview history handler sees it. Replace
            // just those two entries, retaining native clipboard/window actions.
            // In pinned Tauri 2.12 these are the first two Edit items.
            for _ in 0..2 {
                edit.remove_at(0)?;
            }
            edit.insert_items(
                &[
                    &MenuItem::with_id(app, "incant.undo", "Undo", true, Some("Cmd+Z"))?,
                    &MenuItem::with_id(app, "incant.redo", "Redo", true, Some("Cmd+Shift+Z"))?,
                ],
                0,
            )?;
            break;
        }
    }
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        let action = match event.id().as_ref() {
            "incant.undo" => "undo",
            "incant.redo" => "redo",
            _ => return,
        };
        // The UI decides between a text draft and project history. Never change
        // the document directly here, or a shortcut could undo behind a dialog.
        if let Err(error) = app.emit_to(
            EventTarget::webview("editor"),
            "incant:history-request",
            action,
        ) {
            eprintln!("Could not route history menu action: {error}");
        }
    });
    Ok(())
}
