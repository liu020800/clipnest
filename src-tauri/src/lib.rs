mod ai;
mod database;
mod tags;

use database::Database;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tags::auto_tag;
use tauri::{
    Emitter,
    menu::{MenuBuilder, MenuItemBuilder, Submenu, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent, Runtime,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_single_instance::init as single_instance_init;

struct AppState {
    db: Mutex<Database>,
    last_clipboard: Mutex<String>,
    quitting: AtomicBool,
}

const TRAY_ID: &str = "main-tray";

#[tauri::command]
fn copy_to_clipboard(app: tauri::AppHandle, content: String) -> Result<(), String> {
    app.clipboard().write_text(content).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_snippet(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    title: String,
    content: String,
    tags: Option<String>,
) -> Result<i64, String> {
    let final_tags = tags.or_else(|| auto_tag(&content, &title));
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = db
        .insert_snippet(&title, &content, final_tags.as_deref())
        .map_err(|e| e.to_string())?;
    drop(db);
    let _ = refresh_tray_menu(&app);
    Ok(id)
}

#[tauri::command]
fn search_snippets(
    state: tauri::State<AppState>,
    query: String,
) -> Result<Vec<database::Snippet>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.search(&query, 50).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_clipboard_content(state: tauri::State<AppState>) -> Result<String, String> {
    state
        .last_clipboard
        .lock()
        .map(|c| c.clone())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snippet(app: tauri::AppHandle, state: tauri::State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_snippet(id).map_err(|e| e.to_string())?;
    drop(db);
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
fn toggle_pin(app: tauri::AppHandle, state: tauri::State<AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.toggle_pin(id).map_err(|e| e.to_string())?;
    drop(db);
    let _ = refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
async fn auto_tag_ai(title: String, content: String) -> Result<String, String> {
    ai::ollama_tag(&content, &title).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(single_instance_init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("search") {
                let _ = w.show();
                let _ = w.set_focus();
            } else {
                show_search_window(app);
            }
        }))
        .setup(|app| {
            let app_handle = app.handle().clone();

            let db_path = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir")
                .join("copyliusq.db");

            std::fs::create_dir_all(db_path.parent().unwrap()).ok();

            let db = Database::new(db_path.to_str().unwrap())
                .expect("failed to open database");

            let _ = db.init_settings();
            app.manage(AppState {
                db: Mutex::new(db),
                last_clipboard: Mutex::new(String::new()),
                quitting: AtomicBool::new(false),
            });

            setup_tray(&app_handle)?;

            register_shortcuts(&app_handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            copy_to_clipboard,
            save_snippet,
            search_snippets,
            get_clipboard_content,
            delete_snippet,
            toggle_pin,
            auto_tag_ai,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let RunEvent::ExitRequested { api, .. } = event {
                if !_app_handle.state::<AppState>().quitting.load(Ordering::Relaxed) {
                    api.prevent_exit();
                }
            }
        });
}

fn setup_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "打开搜索")
        .accelerator("Alt+Space")
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;

    let pinned_submenu = build_snippet_submenu::<R>(app, "固定收藏", true, 5);
    let recent_submenu = build_snippet_submenu::<R>(app, "最近保存", false, 5);

    let menu = MenuBuilder::new(app)
        .item(&show)
        .separator()
        .item(&pinned_submenu)
        .separator()
        .item(&recent_submenu)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref().to_string();
            if id == "show" {
                show_search_window(app);
            } else if id == "quit" {
                if let Some(state) = app.try_state::<AppState>() {
                    state.quitting.store(true, Ordering::Relaxed);
                }
                app.exit(0);
            } else if let Some(snippet_id) = id.strip_prefix("snippet-") {
                if let Ok(id_num) = snippet_id.parse::<i64>() {
                    copy_snippet_to_clipboard::<R>(app, id_num);
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_search_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn refresh_tray_menu<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "打开搜索")
        .accelerator("Alt+Space")
        .build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let pinned_submenu = build_snippet_submenu::<R>(app, "固定收藏", true, 5);
    let recent_submenu = build_snippet_submenu::<R>(app, "最近保存", false, 5);

    let menu = MenuBuilder::new(app)
        .item(&show)
        .separator()
        .item(&pinned_submenu)
        .separator()
        .item(&recent_submenu)
        .separator()
        .item(&quit)
        .build()?;

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))?;
    }

    Ok(())
}

fn build_snippet_submenu<R: Runtime>(
    app: &tauri::AppHandle<R>,
    label: &str,
    pinned_only: bool,
    limit: i64,
) -> Submenu<R> {
    let mut builder = SubmenuBuilder::new(app, label);

    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(db) = state.db.lock() {
            let snippets = if pinned_only {
                db.get_pinned(limit).unwrap_or_default()
            } else {
                db.get_recent(limit).unwrap_or_default()
            };

            if snippets.is_empty() {
                builder = builder.text("none", if pinned_only { "暂无固定内容" } else { "暂无最近内容" });
            } else {
                for snippet in &snippets {
                    let id_str = format!("snippet-{}", snippet.id);
                    let title = if snippet.title.chars().count() > 30 {
                        format!("{}...", snippet.title.chars().take(27).collect::<String>())
                    } else {
                        snippet.title.clone()
                    };
                    builder = builder.text(&id_str, &title);
                }
            }
        }
    } else {
        builder = builder.text("none", "暂无内容");
    }

    builder.build().expect("failed to build snippet submenu")
}

fn copy_snippet_to_clipboard<R: Runtime>(app: &tauri::AppHandle<R>, id: i64) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(db) = state.db.lock() {
            if let Ok(Some(snippet)) = db.get_snippet_by_id(id) {
                drop(db);
                let _ = app.clipboard().write_text(snippet.content);
            }
        }
    }
}

fn register_shortcuts<R: Runtime>(app: &tauri::AppHandle<R>) {
    let _ = app.global_shortcut().on_shortcut(
        Shortcut::new(Some(Modifiers::ALT), Code::KeyW),
        move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                handle_save_shortcut(app);
            }
        },
    );

    let _ = app.global_shortcut().on_shortcut(
        Shortcut::new(Some(Modifiers::ALT), Code::Space),
        move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                show_search_window(app);
            }
        },
    );
}

fn handle_save_shortcut<R: Runtime>(app: &tauri::AppHandle<R>) {
    let content = match app.clipboard().read_text() {
        Ok(text) => text,
        Err(_) => {
            let _ = app.emit("notification", "读取剪贴板失败");
            return;
        }
    };

    if content.trim().is_empty() {
        return;
    }

    if let Ok(mut last) = app.state::<AppState>().last_clipboard.lock() {
        *last = content.clone();
    }

    let windows = app.webview_windows();
    if let Some(w) = windows.get("save") {
        let _ = w.show();
        let _ = w.set_focus();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            app,
            "save",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("快速保存")
        .inner_size(560.0, 500.0)

        .min_inner_size(500.0, 440.0)

        .max_inner_size(640.0, 600.0)
        .center()
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .focused(true)
        .resizable(false)
        .transparent(true)
        .build();
    }
}

fn show_search_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    let windows = app.webview_windows();
    if let Some(w) = windows.get("search") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.unminimize();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            app,
            "search",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("ClipNest 搜索")
        .inner_size(900.0, 580.0)
        .min_inner_size(700.0, 420.0)
        .center()
        .decorations(false)
        .skip_taskbar(true)
        .transparent(true)
        .build();
    }
}
