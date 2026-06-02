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
use tauri_plugin_autostart::ManagerExt;

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

#[tauri::command]
fn update_snippet(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    id: i64,
    title: Option<String>,
    tags: Option<String>,
    pinned: Option<bool>,
) -> Result<Option<database::Snippet>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db
        .update_snippet(
            id,
            title.as_deref(),
            tags.as_deref(),
            pinned,
        )
        .map_err(|e| e.to_string())?;
    drop(db);
    let _ = refresh_tray_menu(&app);
    Ok(result)
}

#[tauri::command]
fn export_markdown(state: tauri::State<AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let snippets = db.get_all_snippets().map_err(|e| e.to_string())?;
    drop(db);

    let now = chrono::Local::now();
    let filename = format!("clipnest-export-{}.md", now.format("%Y%m%d-%H%M%S"));
    let export_dir = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "无法获取文档目录".to_string())?
        .join("ClipNest");
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    let export_path = export_dir.join(&filename);

    let mut md = String::new();
    md.push_str(&format!("# ClipNest 知识库导出\n\n"));
    md.push_str(&format!("导出时间: {}\n\n", now.format("%Y-%m-%d %H:%M:%S")));
    md.push_str(&format!("共 {} 条记录\n\n---\n\n", snippets.len()));

    for s in &snippets {
        md.push_str(&format!("## {}\n\n", s.title));
        if s.pinned {
            md.push_str("> 📌 已置顶\n\n");
        }
        md.push_str(&format!("- **类型**: {}\n", s.snippet_type.as_deref().unwrap_or("text")));
        md.push_str(&format!("- **创建**: {}\n", s.created_at));
        md.push_str(&format!("- **更新**: {}\n", s.updated_at));
        if let Some(tags) = &s.tags {
            if !tags.is_empty() {
                let tag_list: Vec<String> = tags.split(',').map(|t| format!("#{}", t.trim())).collect();
                md.push_str(&format!("- **标签**: {}\n", tag_list.join(" ")));
            }
        }
        md.push_str("\n");

        let lang = match s.snippet_type.as_deref() {
            Some("url") => "",
            Some("code") => detect_code_lang(&s.content),
            _ => "",
        };
        md.push_str(&format!("```{}\n{}\n```\n\n", lang, s.content));
        md.push_str("---\n\n");
    }

    std::fs::write(&export_path, md).map_err(|e| e.to_string())?;
    Ok(export_path.to_string_lossy().to_string())
}

fn detect_code_lang(content: &str) -> &'static str {
    let lower = content.to_lowercase();
    if lower.contains("fn ") || lower.contains("let ") || lower.contains("->") {
        "rust"
    } else if lower.contains("def ") || lower.contains("import ") || lower.contains("print(") {
        "python"
    } else if lower.contains("function ") || lower.contains("const ") || lower.contains("=>") {
        "javascript"
    } else if lower.contains("docker") || content.starts_with("#!/bin/") {
        "bash"
    } else if lower.contains("select ") || lower.contains("from ") || lower.contains("where ") {
        "sql"
    } else {
        ""
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
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
            update_snippet,
            export_markdown,
            open_settings,
            save_setting,
            get_all_settings,
            get_autostart,
            set_autostart,
            save_image,
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
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app)?;
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
        .item(&settings)
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
            } else if id == "settings" {
                show_settings_window(app);
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
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app)?;
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
        .item(&settings)
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

fn show_settings_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    let windows = app.webview_windows();
    if let Some(w) = windows.get("settings") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.unminimize();
    } else {
        let _ = tauri::WebviewWindowBuilder::new(
            app,
            "settings",
            tauri::WebviewUrl::App("index.html".into()),
        )
        .title("ClipNest 设置")
        .inner_size(560.0, 540.0)
        .min_inner_size(560.0, 540.0)
        .center()
        .decorations(false)
        .skip_taskbar(true)
        .always_on_top(false)
        .focused(true)
        .resizable(false)
        .transparent(true)
        .build();
    }
}

#[tauri::command]
fn open_settings(app: tauri::AppHandle) {
    show_settings_window(&app);
}

#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    if enable {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn save_image(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    title: String,
    tags: Option<String>,
    image_bytes: Vec<u8>,
    ext: String,
) -> Result<i64, String> {
    use base64::{engine::general_purpose, Engine as _};

    let bytes = if image_bytes.iter().all(|b| b.is_ascii()) && image_bytes.len() > 1000 {
        match general_purpose::STANDARD.decode(&image_bytes) {
            Ok(b) => b,
            Err(_) => image_bytes,
        }
    } else {
        image_bytes
    };

    let image_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("images");
    std::fs::create_dir_all(&image_dir).map_err(|e| e.to_string())?;

    let filename = format!("img-{}.{}", chrono::Local::now().format("%Y%m%d-%H%M%S-%3f"), ext.trim_start_matches('.'));
    let image_path = image_dir.join(&filename);
    std::fs::write(&image_path, &bytes).map_err(|e| e.to_string())?;

    let final_tags = tags.or_else(|| auto_tag("", &title));
    let content = format!("[图片] {}", filename);
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = db
        .insert_snippet_with_image(
            &title,
            &content,
            final_tags.as_deref(),
            &image_path.to_string_lossy(),
        )
        .map_err(|e| e.to_string())?;
    drop(db);
    let _ = refresh_tray_menu(&app);
    Ok(id)
}

#[tauri::command]
fn save_setting(
    state: tauri::State<AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_all_settings(state: tauri::State<AppState>) -> Result<std::collections::HashMap<String, String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let autostart = db.get_setting("autostart").map_err(|e| e.to_string())?.unwrap_or_else(|| "false".to_string());
    let capture_shortcut = db.get_setting("capture_shortcut").map_err(|e| e.to_string())?.unwrap_or_else(|| "Alt+W".to_string());
    let search_shortcut = db.get_setting("search_shortcut").map_err(|e| e.to_string())?.unwrap_or_else(|| "Alt+Space".to_string());

    let mut map = std::collections::HashMap::new();
    map.insert("autostart".to_string(), autostart);
    map.insert("capture_shortcut".to_string(), capture_shortcut);
    map.insert("search_shortcut".to_string(), search_shortcut);
    Ok(map)
}
