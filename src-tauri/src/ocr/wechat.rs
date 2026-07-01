use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use tauri::{AppHandle, Manager};

use super::{OcrEngine, OcrError, OcrResult, OCR_TIMEOUT};

const DEV_EXE_REL: &str = "scripts/ocr_host/bin/Release/net8.0/wcocr.exe";
const RESOURCE_EXE_REL: &str = "ocr_host/wcocr.exe";

fn exe_has_data(exe: &Path) -> bool {
    exe.parent()
        .map(|dir| dir.join("wco_data").join("WeChatOCR.exe").exists())
        .unwrap_or(false)
}

fn resolve_exe_path(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidates = [
                parent.join(DEV_EXE_REL),
                parent.join("..").join("..").join(DEV_EXE_REL),
                parent.join("..").join("..").join("..").join(DEV_EXE_REL),
            ];
            for candidate in candidates {
                if candidate.exists() && exe_has_data(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join(DEV_EXE_REL);
        if candidate.exists() && exe_has_data(&candidate) {
            return Some(candidate);
        }
    }

    if let Ok(dir) = app.path().resource_dir() {
        // Tauri v2 打包时 bundle.resources 配置写的是 "resources/ocr_host/**/*",
        // 这个相对结构会原样保留到 resource_dir() 下,所以真实路径是
        // resource_dir()/resources/ocr_host/wcocr.exe(多一层 "resources")。
        // 老代码只试了不带 "resources" 的路径,导致 release 下永远找不到引擎。
        // 这里两种都试,兼容不同打包布局。
        let candidates = [
            dir.join("resources").join(RESOURCE_EXE_REL),
            dir.join(RESOURCE_EXE_REL),
        ];
        for candidate in candidates {
            if candidate.exists() && exe_has_data(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

pub struct WechatOcrEngine {
    exe: Option<PathBuf>,
}

impl WechatOcrEngine {
    pub fn with_app(app: &AppHandle) -> Self {
        Self {
            exe: resolve_exe_path(app),
        }
    }

    pub fn exe_path(&self) -> Option<&Path> {
        self.exe.as_deref()
    }
}

impl OcrEngine for WechatOcrEngine {
    fn name(&self) -> &'static str {
        "wechatocr"
    }

    fn available(&self) -> bool {
        self.exe.is_some()
    }

    fn recognize(&self, image_path: &Path) -> Result<OcrResult, OcrError> {
        let exe = self.exe.as_ref().ok_or_else(|| {
            OcrError::EngineUnavailable("未找到可用的 WeChatOCR 引擎文件".into())
        })?;
        if !image_path.exists() {
            return Err(OcrError::BadOutput(format!(
                "图片不存在: {}",
                image_path.display()
            )));
        }

        let started = Instant::now();
        let mut cmd = Command::new(exe);
        cmd.arg(image_path)
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        if let Some(dir) = exe.parent() {
            cmd.current_dir(dir);
        }
        let mut child = cmd.spawn().map_err(|e| OcrError::SpawnFailed(e.to_string()))?;

        // 同 rapidocr: Windows pipe buffer ~64KB,必须 drain stdout/stderr,否则子进程会阻塞写入
        // 主线程的 try_wait 永远拿不到 Some(_) → 必然 30s 超时。
        let stdout_pipe = child.stdout.take().ok_or_else(|| {
            OcrError::SpawnFailed("子进程 stdout 未配置为 piped".into())
        })?;
        let stderr_pipe = child.stderr.take().ok_or_else(|| {
            OcrError::SpawnFailed("子进程 stderr 未配置为 piped".into())
        })?;
        let stdout_handle = std::thread::spawn(move || {
            use std::io::Read;
            let mut pipe = stdout_pipe;
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        });
        let stderr_handle = std::thread::spawn(move || {
            use std::io::Read;
            let mut pipe = stderr_pipe;
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        });

        let exit_status = loop {
            if started.elapsed() > OCR_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_handle.join();
                let _ = stderr_handle.join();
                return Err(OcrError::Timeout);
            }
            match child
                .try_wait()
                .map_err(|e| OcrError::SpawnFailed(e.to_string()))?
            {
                Some(status) => break status,
                None => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        };

        let stdout_bytes = stdout_handle.join().unwrap_or_default();
        let stderr_bytes = stderr_handle.join().unwrap_or_default();

        if !exit_status.success() {
            return Err(OcrError::NonZeroExit {
                code: exit_status.code(),
                stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&stdout_bytes);
        parse_wechat_output(&stdout)
    }
}

pub fn parse_wechat_output(stdout: &str) -> Result<OcrResult, OcrError> {
    let json = stdout
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| line.starts_with('{') && line.ends_with('}'))
        .ok_or_else(|| OcrError::BadOutput("未在 WeChatOCR 输出中找到 JSON 行".into()))?;
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| OcrError::BadOutput(format!("WeChatOCR JSON 解析失败: {e}")))?;
    let text = value
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .replace("\r\n", "\n")
        .trim()
        .to_string();
    Ok(OcrResult {
        text,
        source: "wechatocr".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_json_text() {
        let r = parse_wechat_output(r#"{"text":"中文识别\r\nHello"}"#).unwrap();
        assert_eq!(r.text, "中文识别\nHello");
        assert_eq!(r.source, "wechatocr");
    }
}
