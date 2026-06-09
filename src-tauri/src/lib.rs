mod ai;
pub mod clipboard;
pub mod commands;
pub mod database;
mod hotkeys;
pub mod ocr;
mod search;
mod settings;
mod tags;
mod tray;

use database::Database;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

pub struct AppState {
    pub db: Mutex<Database>,
    pub last_clipboard: Mutex<String>,
    pub pending_capture_text: Mutex<Option<String>>,
    pub quitting: AtomicBool,
}

fn build_window<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    label: &str,
) -> Result<(), tauri::Error> {
    let mut builder =
        tauri::WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("index.html".into()))
            .inner_size(560.0, 500.0)
            .center()
            .decorations(false)
            .skip_taskbar(true)
            .transparent(true)
            .visible(false);

    match label {
        "search" => {
            builder = builder
                .title("ClipNest 搜索")
                .inner_size(900.0, 580.0)
                .min_inner_size(700.0, 420.0);
        }
        "settings" => {
            builder = builder
                .title("ClipNest 设置")
                .inner_size(560.0, 540.0)
                .min_inner_size(560.0, 540.0)
                .always_on_top(false)
                .resizable(false);
        }
        "capture" => {
            builder = builder
                .title("快速保存")
                .inner_size(560.0, 500.0)
                .min_inner_size(500.0, 440.0)
                .max_inner_size(640.0, 600.0)
                .always_on_top(true)
                .resizable(false);
        }
        "screen_ocr" => {
            builder = builder
                .title("框选识别")
                .inner_size(1280.0, 720.0)
                .maximized(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .decorations(false)
                .transparent(true)
                .resizable(false);
        }
        _ => {}
    }

    builder.build()?;
    Ok(())
}

pub fn show_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));

    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        show_window(app, "search");
    }));

    builder
        .setup(|app| {
            let app_handle = app.handle().clone();

            let db_path = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir")
                .join("copyliusq.db");

            std::fs::create_dir_all(db_path.parent().unwrap()).ok();

            // Backup before opening if a pre-existing DB is found.
            if db_path.exists() {
                let backups_dir = db_path.parent().unwrap().join("backups");
                if let Err(e) = database::backup_database_file(&db_path, &backups_dir) {
                    eprintln!("[setup] backup before migration failed: {e}");
                }
            }

            let db = Database::new(db_path.to_str().unwrap()).expect("failed to open database");

            let _ = db.init_settings();
            settings::seed_defaults(&db);
            let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            let _ = db.save_setting(
                "autostart",
                if autostart_enabled { "true" } else { "false" },
            );
            app.manage(AppState {
                db: Mutex::new(db),
                last_clipboard: Mutex::new(String::new()),
                pending_capture_text: Mutex::new(None),
                quitting: AtomicBool::new(false),
            });

            // Pre-create all windows to avoid on-demand creation freezes
            for label in ["search", "settings", "capture", "screen_ocr"] {
                if let Err(e) = build_window(&app_handle, label) {
                    eprintln!("[setup] failed to create window '{label}': {e}");
                }
            }

            tray::setup_tray(&app_handle)?;

            hotkeys::register_shortcuts(&app_handle);

            show_window(&app_handle, "search");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // clipboard
            clipboard::copy_to_clipboard,
            clipboard::get_clipboard_content,
            clipboard::get_current_clipboard_text,
            // search
            search::search_snippets,
            search::list_snippets,
            // commands
            commands::save_snippet,
            commands::save_snippet_force,
            commands::delete_snippet,
            commands::toggle_pin,
            commands::auto_tag_ai,
            commands::update_snippet,
            commands::export_markdown,
            commands::export_json,
            commands::import_json,
            commands::open_settings,
            commands::open_capture,
            commands::hide_window,
            commands::capture_screen_ocr_region,
            commands::set_pending_capture_text,
            commands::take_pending_capture_text,
            commands::get_ocr_capability,
            commands::resolve_image_path,
            commands::get_autostart,
            commands::set_autostart,
            commands::save_setting,
            commands::get_all_settings,
            commands::get_db_path,
            commands::open_db_dir,
            commands::backup_database,
            commands::list_tags,
            commands::rename_tag,
            commands::delete_tag,
            commands::merge_tags,
            // hotkeys
            hotkeys::update_shortcut,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !_app_handle
                    .state::<AppState>()
                    .quitting
                    .load(Ordering::Relaxed)
                {
                    api.prevent_exit();
                }
            }
        });
}
