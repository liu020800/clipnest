use tauri::{Manager, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

const HISTORY_POLL_INTERVAL_MS: u64 = 700;
const HISTORY_TITLE_MAX_CHARS: usize = 30;

pub fn write_text_to_clipboard<R: Runtime>(
    app: &tauri::AppHandle<R>,
    text: impl AsRef<str>,
) -> Result<(), String> {
    let text = text.as_ref();
    let mut last_error = String::new();

    for attempt in 0..3 {
        match app.clipboard().write_text(text) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last_error = err.to_string();
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_millis(40));
                }
            }
        }
    }

    Err(format!("写入剪贴板失败: {last_error}"))
}

#[tauri::command]
pub fn copy_to_clipboard<R: Runtime>(app: tauri::AppHandle<R>, text: String) -> Result<(), String> {
    write_text_to_clipboard(&app, text)
}

#[tauri::command]
pub fn get_current_clipboard_text<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<crate::AppState>,
) -> Result<String, String> {
    let text = app
        .clipboard()
        .read_text()
        .map_err(|e| format!("读取剪贴板失败: {e}"))?;
    // 同步缓存: 让 get_clipboard_content(后台兜底接口)有值可读,
    // 避免 webview 失去焦点时主接口因权限被拒而失败。
    if let Ok(mut cache) = state.last_clipboard.lock() {
        *cache = text.clone();
    }
    Ok(text)
}

#[tauri::command]
pub fn get_clipboard_content(state: tauri::State<crate::AppState>) -> Result<String, String> {
    state
        .last_clipboard
        .lock()
        .map(|c| c.clone())
        .map_err(|e| e.to_string())
}

pub fn start_clipboard_history_monitor(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last_seen = String::new();
        let mut initialized = false;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(HISTORY_POLL_INTERVAL_MS));

            let Ok(text) = app.clipboard().read_text() else {
                continue;
            };
            if let Some(state) = app.try_state::<crate::AppState>() {
                if let Ok(mut cache) = state.last_clipboard.lock() {
                    *cache = text.clone();
                }
            }

            if !initialized {
                last_seen = text;
                initialized = true;
                continue;
            }
            if text == last_seen {
                continue;
            }
            last_seen = text.clone();

            let trimmed = text.trim();
            if trimmed.is_empty() {
                continue;
            }

            let Some(state) = app.try_state::<crate::AppState>() else {
                continue;
            };
            let Ok(db) = state.db.lock() else {
                continue;
            };
            let settings = crate::settings::Settings::load(&db);
            if !settings.clipboard_history_enabled {
                continue;
            }
            if trimmed.chars().count() > settings.capture_text_max_length {
                continue;
            }

            let title = clipboard_history_title(trimmed, HISTORY_TITLE_MAX_CHARS);
            if let Err(e) =
                db.upsert_clipboard_history(&title, trimmed, settings.clipboard_history_max)
            {
                eprintln!("[clipboard-history] failed to save clipboard text: {e}");
            }
        }
    });
}

fn clipboard_history_title(text: &str, max_chars: usize) -> String {
    let first_line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("剪贴板文本");
    let collapsed = first_line.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.chars().take(max_chars.max(1)).collect()
}

#[cfg(test)]
mod tests {
    use super::clipboard_history_title;

    #[test]
    fn clipboard_history_title_uses_first_non_empty_line() {
        assert_eq!(
            clipboard_history_title("\n  hello   world\nsecond", 30),
            "hello world"
        );
    }

    #[test]
    fn clipboard_history_title_has_a_fallback() {
        assert_eq!(clipboard_history_title("  \n\t", 30), "剪贴板文本");
    }
}
