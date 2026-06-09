use tauri::Runtime;
use tauri_plugin_clipboard_manager::ClipboardExt;

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
pub fn copy_to_clipboard<R: Runtime>(
    app: tauri::AppHandle<R>,
    text: String,
) -> Result<(), String> {
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
