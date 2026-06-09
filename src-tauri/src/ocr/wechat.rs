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
        let candidate = dir.join(RESOURCE_EXE_REL);
        if candidate.exists() && exe_has_data(&candidate) {
            return Some(candidate);
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
        if let Some(dir) = exe.parent() {
            cmd.current_dir(dir);
        }
        let mut child = cmd.spawn().map_err(|e| OcrError::SpawnFailed(e.to_string()))?;

        loop {
            if started.elapsed() > OCR_TIMEOUT {
                let _ = child.kill();
                return Err(OcrError::Timeout);
            }
            match child
                .try_wait()
                .map_err(|e| OcrError::SpawnFailed(e.to_string()))?
            {
                Some(_) => break,
                None => std::thread::sleep(std::time::Duration::from_millis(50)),
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|e| OcrError::SpawnFailed(e.to_string()))?;
        if !output.status.success() {
            return Err(OcrError::NonZeroExit {
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
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
