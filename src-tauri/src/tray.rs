use std::sync::atomic::Ordering;
use tauri::menu::{MenuBuilder, MenuItemBuilder, Submenu, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, Runtime};

pub const TRAY_ID: &str = "main-tray";

pub fn setup_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
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
                crate::show_window(app, "search");
            } else if id == "settings" {
                crate::show_window(app, "settings");
            } else if id == "quit" {
                if let Some(state) = app.try_state::<crate::AppState>() {
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
                crate::show_window(tray.app_handle(), "search");
            }
        })
        .build(app)?;

    Ok(())
}

pub fn refresh_tray_menu<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
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

    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(db) = state.db.lock() {
            let snippets = if pinned_only {
                db.get_pinned(limit).unwrap_or_default()
            } else {
                db.get_recent(limit).unwrap_or_default()
            };

            if snippets.is_empty() {
                builder = builder.text(
                    "none",
                    if pinned_only {
                        "暂无固定内容"
                    } else {
                        "暂无最近内容"
                    },
                );
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
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(db) = state.db.lock() {
            if let Ok(Some(snippet)) = db.get_snippet_by_id(id) {
                drop(db);
                let _ = crate::clipboard::write_text_to_clipboard(app, snippet.content);
            }
        }
    }
}
