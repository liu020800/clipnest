use crate::database::Database;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::{Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn apply_shortcut_bindings<R: Runtime>(app: &tauri::AppHandle<R>) {
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().unwrap();
    let capture = db
        .get_setting("capture_shortcut")
        .unwrap_or(None)
        .unwrap_or_else(|| "Ctrl+Shift+S".to_string());
    let capture_alt = db
        .get_setting("capture_shortcut_alt")
        .unwrap_or(None)
        .unwrap_or_else(|| "Alt+W".to_string());
    let search = db
        .get_setting("search_shortcut")
        .unwrap_or(None)
        .unwrap_or_else(|| "Alt+Space".to_string());
    let screen_ocr = db
        .get_setting("screen_ocr_shortcut")
        .unwrap_or(None)
        .unwrap_or_else(|| "Ctrl+Shift+O".to_string());
    drop(db);

    let _ = app.global_shortcut().unregister_all();

    if let Ok(shortcut) = Shortcut::from_str(&capture) {
        let _ = app
            .global_shortcut()
            .on_shortcut(shortcut, move |app, _s, event| {
                if event.state == ShortcutState::Pressed {
                    handle_save_shortcut(app);
                }
            });
    }

    if !capture_alt.trim().is_empty() {
        if let Ok(shortcut) = Shortcut::from_str(&capture_alt) {
            let _ = app
                .global_shortcut()
                .on_shortcut(shortcut, move |app, _s, event| {
                    if event.state == ShortcutState::Pressed {
                        handle_save_shortcut(app);
                    }
                });
        }
    }

    if let Ok(shortcut) = Shortcut::from_str(&search) {
        let _ = app
            .global_shortcut()
            .on_shortcut(shortcut, move |app, _s, event| {
                if event.state == ShortcutState::Pressed {
                    crate::show_window(app, "search");
                }
            });
    }

    if !screen_ocr.trim().is_empty() {
        if let Ok(shortcut) = Shortcut::from_str(&screen_ocr) {
            let _ = app
                .global_shortcut()
                .on_shortcut(shortcut, move |app, _s, event| {
                    if event.state == ShortcutState::Pressed {
                        crate::show_window(app, "screen_ocr");
                    }
                });
        }
    }
}

pub fn register_shortcuts<R: Runtime>(app: &tauri::AppHandle<R>) {
    apply_shortcut_bindings(app);
}

pub fn handle_save_shortcut<R: Runtime>(app: &tauri::AppHandle<R>) {
    crate::show_window(app, "capture");
}

#[tauri::command]
pub fn update_shortcut<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<crate::AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    if key != "capture_shortcut"
        && key != "capture_shortcut_alt"
        && key != "search_shortcut"
        && key != "screen_ocr_shortcut"
    {
        return Err(format!("未知设置键: {key}"));
    }
    // capture_shortcut 是主保存键,不允许空;其余两个可空 = 禁用。
    if value.trim().is_empty() && key == "capture_shortcut" {
        return Err("快捷键不能为空".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_setting(&key, value.trim())
        .map_err(|e| e.to_string())?;
    drop(db);
    apply_shortcut_bindings(&app);
    Ok(())
}

// Helper to keep trait bounds visible
pub fn _unused_db_mutex() -> Option<Mutex<Database>> {
    None
}
