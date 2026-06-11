use std::time::Duration;
use tauri::{Manager, Runtime};

#[cfg(target_os = "windows")]
mod platform {
    use std::{mem, thread, time::Duration};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

    pub fn foreground_window() -> Option<isize> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_null() {
            None
        } else {
            Some(hwnd as isize)
        }
    }

    pub fn paste_into_window(hwnd: Option<isize>) -> Result<(), String> {
        if let Some(hwnd) = hwnd {
            let hwnd = hwnd as HWND;
            if !hwnd.is_null() {
                unsafe {
                    SetForegroundWindow(hwnd);
                }
                thread::sleep(Duration::from_millis(90));
            }
        }

        send_ctrl_v()
    }

    fn key_input(vk: u16, flags: u32) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn send_ctrl_v() -> Result<(), String> {
        let inputs = [
            key_input(VK_CONTROL, 0),
            key_input(VK_V, 0),
            key_input(VK_V, KEYEVENTF_KEYUP),
            key_input(VK_CONTROL, KEYEVENTF_KEYUP),
        ];
        let sent = unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                mem::size_of::<INPUT>() as i32,
            )
        };
        if sent == inputs.len() as u32 {
            Ok(())
        } else {
            Err(format!(
                "模拟粘贴失败: 只发送了 {sent}/{} 个按键事件",
                inputs.len()
            ))
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub fn foreground_window() -> Option<isize> {
        None
    }

    pub fn paste_into_window(_hwnd: Option<isize>) -> Result<(), String> {
        Err("自动粘贴当前仅支持 Windows".to_string())
    }
}

pub fn remember_foreground_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    let Some(hwnd) = platform::foreground_window() else {
        return;
    };
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(mut previous) = state.previous_foreground_window.lock() {
            *previous = Some(hwnd);
        }
    }
}

#[tauri::command]
pub fn paste_text<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<crate::AppState>,
    text: String,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("内容为空,无法粘贴".to_string());
    }

    crate::clipboard::write_text_to_clipboard(&app, &text)?;
    if let Ok(mut cache) = state.last_clipboard.lock() {
        *cache = text;
    }

    if let Some(window) = app.get_webview_window("search") {
        let _ = window.hide();
    }
    std::thread::sleep(Duration::from_millis(70));

    let hwnd = state
        .previous_foreground_window
        .lock()
        .map(|previous| *previous)
        .unwrap_or(None);
    platform::paste_into_window(hwnd)
}
