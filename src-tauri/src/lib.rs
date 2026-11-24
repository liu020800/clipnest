mod ai;
pub mod clipboard;
pub mod commands;
pub mod database;
mod hotkeys;
pub mod ocr;
mod paste;
mod search;
mod settings;
mod tags;
mod tray;

use database::Database;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

pub struct AppState {
    pub db: Mutex<Database>,
    pub last_clipboard: Mutex<String>,
    pub pending_capture_text: Mutex<Option<String>>,
    pub previous_foreground_window: Mutex<Option<isize>>,
    pub quitting: AtomicBool,
    // 记录"应用自身写入剪贴板"的最近一次内容与时间戳,用于抑制
    // start_clipboard_history_monitor 把自己写入的文本再当成"用户新复制"
    // 重新插入历史(粘贴自循环)。
    pub last_self_write: Mutex<Option<(String, Instant)>>,
}

fn build_window<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    label: &str,
    open_on_startup: bool,
) -> Result<(), tauri::Error> {
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App(window_app_path(label).into()),
    )
    .inner_size(560.0, 500.0)
    .center()
    .decorations(false)
    .skip_taskbar(true)
    .transparent(true)
    .visible(label == "search" && open_on_startup);

    match label {
        "search" => {
            builder = builder
                .title("ClipNest 搜索")
                .inner_size(900.0, 580.0)
                .min_inner_size(700.0, 420.0)
                .always_on_top(window_should_stay_on_top(label));
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
                .always_on_top(window_should_stay_on_top(label))
                .resizable(false);
        }
        "screen_ocr" => {
            builder = builder
                .title("框选识别")
                .fullscreen(true)
                .always_on_top(window_should_stay_on_top(label))
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

fn window_app_path(label: &str) -> String {
    format!("index.html?window={label}")
}

fn window_should_stay_on_top(label: &str) -> bool {
    matches!(label, "search" | "capture" | "screen_ocr")
}

fn should_open_search_on_startup(args: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    !args.into_iter().any(|arg| arg.as_ref() == "--background")
}

pub fn show_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let was_visible = w.is_visible().unwrap_or(false);
        if label == "search" && !was_visible {
            paste::remember_foreground_window(app);
        }
        let _ = w.unminimize();
        let _ = w.show();
        if !was_visible {
            let _ = w.reload();
        }
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
            Some(vec!["--background"]),
        ));

    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
        if should_open_search_on_startup(argv.iter().skip(1)) {
            show_window(app, "search");
        }
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
                let _ = database::backup_database_file(&db_path, &backups_dir);
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
                previous_foreground_window: Mutex::new(None),
                quitting: AtomicBool::new(false),
                last_self_write: Mutex::new(None),
            });

            // Pre-create all windows to avoid on-demand creation freezes
            let open_on_startup = should_open_search_on_startup(std::env::args().skip(1));
            for label in ["search", "settings", "capture", "screen_ocr"] {
                let _ = build_window(&app_handle, label, open_on_startup);
            }

            tray::setup_tray(&app_handle)?;

            hotkeys::register_shortcuts(&app_handle);
            clipboard::start_clipboard_history_monitor(app_handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // clipboard
            clipboard::copy_to_clipboard,
            clipboard::get_clipboard_content,
            clipboard::get_current_clipboard_text,
            paste::paste_text,
            // search
            search::search_snippets,
            search::list_snippets,
            // commands
            commands::save_snippet,
            commands::save_snippet_force,
            commands::delete_snippet,
            commands::toggle_pin,
            commands::copy_snippet_to_clipboard,
            commands::paste_snippet,
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

#[cfg(test)]
mod tests {
    use super::{should_open_search_on_startup, window_app_path, window_should_stay_on_top};

    #[test]
    fn manual_launch_opens_search() {
        assert!(should_open_search_on_startup(std::iter::empty::<&str>()));
    }

    #[test]
    fn autostart_launch_stays_in_background() {
        assert!(!should_open_search_on_startup(["--background"]));
    }

    #[test]
    fn background_flag_must_match_exactly() {
        assert!(should_open_search_on_startup(["--background-other"]));
    }

    #[test]
    fn window_app_path_pins_the_frontend_window_route() {
        assert_eq!(window_app_path("search"), "index.html?window=search");
        assert_eq!(window_app_path("capture"), "index.html?window=capture");
    }

    #[test]
    fn transient_tool_windows_stay_above_foreground_apps() {
        assert!(window_should_stay_on_top("search"));
        assert!(window_should_stay_on_top("capture"));
        assert!(window_should_stay_on_top("screen_ocr"));
        assert!(!window_should_stay_on_top("settings"));
    }
}
