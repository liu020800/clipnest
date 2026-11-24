use tauri::{Manager, Runtime};

const HISTORY_POLL_INTERVAL_MS: u64 = 100;
const HISTORY_SETTLE_DELAY_MS: u64 = 250;
const HISTORY_SAMPLE_INTERVAL_MS: u64 = 80;
const HISTORY_STABLE_SAMPLE_ATTEMPTS: usize = 6;
const HISTORY_TITLE_MAX_CHARS: usize = 30;
const WRITE_VERIFY_INTERVAL_MS: u64 = 25;
const WRITE_VERIFY_ATTEMPTS: usize = 8;
// 写入剪贴板后,这段时间内同样内容被监听线程读到时不会进历史。
// 700ms 轮询 + 70ms 粘贴前置等待 + 安全冗余,2 秒可覆盖单次 copy/paste 链路。
const SELF_WRITE_SUPPRESS_MS: u64 = 2000;

#[cfg(windows)]
const CF_UNICODETEXT: u32 = 13;

fn decode_utf16_text(data: &[u16]) -> Result<String, String> {
    let len = data.iter().position(|unit| *unit == 0).unwrap_or(data.len());
    String::from_utf16(&data[..len]).map_err(|_| "剪贴板文本不是有效的 UTF-16".to_string())
}

#[cfg(windows)]
fn native_read_text() -> Result<String, String> {
    use std::mem::size_of;
    use std::ptr;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

    for _ in 0..8 {
        unsafe {
            if OpenClipboard(ptr::null_mut()) == 0 {
                std::thread::sleep(std::time::Duration::from_millis(15));
                continue;
            }

            let result = if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
                Err("剪贴板没有 Unicode 文本".to_string())
            } else {
                let handle = GetClipboardData(CF_UNICODETEXT);
                if handle.is_null() {
                    Err("读取剪贴板 Unicode 文本失败".to_string())
                } else {
                    let byte_len = GlobalSize(handle);
                    let data_ptr = GlobalLock(handle) as *const u16;
                    if byte_len < size_of::<u16>() || data_ptr.is_null() {
                        Err("锁定剪贴板文本缓冲失败".to_string())
                    } else {
                        let units = byte_len / size_of::<u16>();
                        let data = std::slice::from_raw_parts(data_ptr, units);
                        let text = decode_utf16_text(data);
                        GlobalUnlock(handle);
                        text
                    }
                }
            };
            CloseClipboard();
            return result;
        }
    }

    Err("剪贴板正被其他程序占用".to_string())
}

#[cfg(not(windows))]
fn native_read_text<R: Runtime>(app: &tauri::AppHandle<R>) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .read_text()
        .map_err(|err| format!("读取剪贴板失败: {err}"))
}

#[cfg(windows)]
fn read_clipboard_text<R: Runtime>(
    _app: &tauri::AppHandle<R>,
) -> Result<String, String> {
    native_read_text()
}

#[cfg(not(windows))]
fn read_clipboard_text<R: Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<String, String> {
    native_read_text(app)
}

#[cfg(windows)]
fn native_write_text(text: &str) -> Result<(), String> {
    use std::mem::size_of;
    use std::ptr;
    use windows_sys::Win32::Foundation::GlobalFree;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE, GMEM_ZEROINIT,
    };

    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let byte_len = wide.len() * size_of::<u16>();

    for _ in 0..8 {
        unsafe {
            if OpenClipboard(ptr::null_mut()) == 0 {
                std::thread::sleep(std::time::Duration::from_millis(15));
                continue;
            }

            let memory = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, byte_len);
            if memory.is_null() {
                CloseClipboard();
                return Err("分配剪贴板文本缓冲失败".to_string());
            }
            let data_ptr = GlobalLock(memory) as *mut u16;
            if data_ptr.is_null() {
                GlobalFree(memory);
                CloseClipboard();
                return Err("锁定剪贴板文本缓冲失败".to_string());
            }
            ptr::copy_nonoverlapping(wide.as_ptr(), data_ptr, wide.len());
            GlobalUnlock(memory);

            if EmptyClipboard() == 0 {
                GlobalFree(memory);
                CloseClipboard();
                return Err("清空剪贴板失败".to_string());
            }
            if SetClipboardData(CF_UNICODETEXT, memory).is_null() {
                GlobalFree(memory);
                CloseClipboard();
                return Err("写入剪贴板 Unicode 文本失败".to_string());
            }
            CloseClipboard();
            return Ok(());
        }
    }

    Err("剪贴板正被其他程序占用".to_string())
}

#[cfg(not(windows))]
fn native_write_text<R: Runtime>(
    app: &tauri::AppHandle<R>,
    text: &str,
) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(text)
        .map_err(|err| format!("写入剪贴板失败: {err}"))
}

#[cfg(windows)]
fn write_clipboard_text<R: Runtime>(
    _app: &tauri::AppHandle<R>,
    text: &str,
) -> Result<(), String> {
    native_write_text(text)
}

#[cfg(not(windows))]
fn write_clipboard_text<R: Runtime>(
    app: &tauri::AppHandle<R>,
    text: &str,
) -> Result<(), String> {
    native_write_text(app, text)
}

#[cfg(target_os = "windows")]
fn clipboard_sequence_number() -> Option<u32> {
    let sequence = unsafe {
        windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber()
    };
    (sequence != 0).then_some(sequence)
}

#[cfg(not(target_os = "windows"))]
fn clipboard_sequence_number() -> Option<u32> {
    None
}

fn sequence_changed(previous: &mut Option<u32>, current: Option<u32>) -> bool {
    if current.is_none() || current == *previous {
        return false;
    }
    *previous = current;
    true
}

pub fn write_text_to_clipboard<R: Runtime>(
    app: &tauri::AppHandle<R>,
    text: impl AsRef<str>,
) -> Result<(), String> {
    let text = text.as_ref();
    let mut last_error = String::new();

    for attempt in 0..3 {
        match write_clipboard_text(app, text) {
            Ok(()) => {
                let mut verified = false;
                for _ in 0..WRITE_VERIFY_ATTEMPTS {
                    std::thread::sleep(std::time::Duration::from_millis(
                        WRITE_VERIFY_INTERVAL_MS,
                    ));
                    if read_clipboard_text(app)
                        .map(|actual| actual == text)
                        .unwrap_or(false)
                    {
                        verified = true;
                        break;
                    }
                }
                if !verified {
                    last_error = "写入后校验失败,剪贴板内容未稳定".to_string();
                    continue;
                }

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
    let text = read_clipboard_text(&app)?;
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

fn read_stable_clipboard_text<R: Runtime>(app: &tauri::AppHandle<R>) -> Option<(u32, String)> {
    std::thread::sleep(std::time::Duration::from_millis(HISTORY_SETTLE_DELAY_MS));
    let mut previous: Option<String> = None;

    for _ in 0..HISTORY_STABLE_SAMPLE_ATTEMPTS {
        let sequence_before = clipboard_sequence_number()?;
        let Ok(text) = read_clipboard_text(&app) else {
            std::thread::sleep(std::time::Duration::from_millis(
                HISTORY_SAMPLE_INTERVAL_MS,
            ));
            continue;
        };

        std::thread::sleep(std::time::Duration::from_millis(
            HISTORY_SAMPLE_INTERVAL_MS,
        ));
        let sequence_after = clipboard_sequence_number()?;
        if sequence_before != sequence_after {
            previous = None;
            continue;
        }

        if previous.as_deref() == Some(text.as_str()) {
            return Some((sequence_after, text));
        }
        previous = Some(text);
    }

    None
}

pub fn start_clipboard_history_monitor(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut last_sequence = clipboard_sequence_number();
        let mut last_seen = String::new();
        let mut initialized = false;

        loop {
            std::thread::sleep(std::time::Duration::from_millis(HISTORY_POLL_INTERVAL_MS));

            let sequence = clipboard_sequence_number();
            if !sequence_changed(&mut last_sequence, sequence) {
                continue;
            }

            let Some(state) = app.try_state::<crate::AppState>() else {
                continue;
            };
            let history_enabled = state
                .db
                .lock()
                .map(|db| crate::settings::Settings::load(&db).clipboard_history_enabled)
                .unwrap_or(false);
            if !history_enabled {
                continue;
            }

            // Clipboard providers can use delayed rendering. Require two identical
            // reads with an unchanged sequence before accepting a new history item.
            let Some((settled_sequence, text)) = read_stable_clipboard_text(&app) else {
                continue;
            };
            if Some(settled_sequence) != last_sequence {
                // The sequence changed while stabilizing. Keep the new sequence
                // pending so the next poll reads it instead of dropping it.
                continue;
            }
            last_sequence = Some(settled_sequence);
            if let Ok(mut cache) = state.last_clipboard.lock() {
                *cache = text.clone();
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

            let Ok(db) = state.db.lock() else {
                continue;
            };
            let settings = crate::settings::Settings::load(&db);
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
    use super::{clipboard_history_title, decode_utf16_text, sequence_changed};

    #[cfg(windows)]
    use super::{native_read_text, native_write_text};

    #[test]
    fn clipboard_monitor_reads_only_after_sequence_changes() {
        let mut previous = Some(10);
        assert!(!sequence_changed(&mut previous, Some(10)));
        assert!(sequence_changed(&mut previous, Some(11)));
        assert_eq!(previous, Some(11));
        assert!(!sequence_changed(&mut previous, None));
    }

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

    #[test]
    fn unicode_clipboard_decode_preserves_delimiters_and_tail() {
        let expected = "sample@example.invalid----Demo7X9k----qa37a1b2c3d4e5f6g7h8j9k0m1n2p3q4r5s6t7u8v9w0x1y2z3";
        let mut utf16: Vec<u16> = expected.encode_utf16().collect();
        utf16.push(0);
        assert_eq!(decode_utf16_text(&utf16).unwrap(), expected);
    }

    #[cfg(windows)]
    #[test]
    fn native_clipboard_round_trips_long_alphanumeric_tail() {
        let previous = native_read_text().ok();
        let expected =
            "sample@example.invalid----Demo7X9k----qa37a1b2c3d4e5f6g7h8j9k0m1n2p3q4r5s6t7u8v9w0x1y2z3";

        let result = native_write_text(expected).and_then(|_| native_read_text());
        if let Some(previous) = previous {
            let _ = native_write_text(&previous);
        }

        assert_eq!(result.unwrap(), expected);
    }
}
