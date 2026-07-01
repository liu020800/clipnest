use tauri::{Manager, Runtime};
use tauri_plugin_clipboard_manager::ClipboardExt;

const HISTORY_POLL_INTERVAL_MS: u64 = 700;
const HISTORY_TITLE_MAX_CHARS: usize = 30;
// 写入剪贴板后,这段时间内同样内容被监听线程读到时不会进历史。
// 700ms 轮询 + 70ms 粘贴前置等待 + 安全冗余,2 秒可覆盖单次 copy/paste 链路。
const SELF_WRITE_SUPPRESS_MS: u64 = 2000;

pub fn write_text_to_clipboard<R: Runtime>(
    app: &tauri::AppHandle<R>,
    text: impl AsRef<str>,
) -> Result<(), String> {
    let text = text.as_ref();
    let mut last_error = String::new();

    for attempt in 0..3 {
        match app.clipboard().write_text(text) {
            Ok(()) => {
                // 标记自写: 让 start_clipboard_history_monitor 在 SELF_WRITE_SUPPRESS_MS 内
                // 看到相同 text 时直接跳过,避免「从历史粘贴 → 自己又被插回顶部」的回环。
                if let Some(state) = app.try_state::<crate::AppState>() {
                    if let Ok(mut slot) = state.last_self_write.lock() {
                        *slot = Some((text.to_string(), std::time::Instant::now()));
                    }
                }
                return Ok(());
            }
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

            // 抑制窗口检查: 如果这段文本是我们刚 write_text_to_clipboard 写进去的
            // (典型场景: 用户从搜索历史 Enter 粘贴),不要再当成一次「新复制」插历史。
            if let Some(state) = app.try_state::<crate::AppState>() {
                if let Ok(mut slot) = state.last_self_write.lock() {
                    if let Some((cached, ts)) = slot.as_ref() {
                        if cached == &text
                            && ts.elapsed()
                                < std::time::Duration::from_millis(SELF_WRITE_SUPPRESS_MS)
                        {
                            // 命中抑制: 顺手清掉标记,避免长期占用。
                            *slot = None;
                            continue;
                        }
                        // 过期标记顺带清理。
                        if ts.elapsed()
                            >= std::time::Duration::from_millis(SELF_WRITE_SUPPRESS_MS)
                        {
                            *slot = None;
                        }
                    }
                }
            }

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
            let _ = db.upsert_clipboard_history(&title, trimmed, settings.clipboard_history_max);
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
