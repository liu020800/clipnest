// RapidOCR 引擎(v1.1)
// 通过 Python 子进程调用 RapidOCR (PaddleOCR-based, 离线, 不依赖本地微信).
// 协议与原 wcocr.exe 兼容: --json 输出 {"text": "..."}.
//
// 资源要求: 宿主系统装了 Python 3.8+, 首次运行子进程会自举 pip install rapidocr onnxruntime.
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use tauri::{AppHandle, Manager};

use super::OcrEngine;
use super::{OcrError, OcrResult, OCR_TIMEOUT};

const DEV_SCRIPT_REL: &str = "scripts/ocr_host/rcr_ocr.py";
const TAURI_RESOURCE_SCRIPT_REL: &str = "resources/rapidocr/rcr_ocr.py";
const SUBDIR: &str = "rapidocr";

/// 寻找 Python 解释器 (按优先级).
/// 1) 用户明确设置 CLIPNEST_PYTHON
/// 2) PATH 中的 py.exe / python.exe
/// 3) 已知常见安装位置
pub fn find_python() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("CLIPNEST_PYTHON") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            return Some(pb);
        }
    }
    for cmd in &["py", "python", "python3"] {
        if let Ok(out) = std::process::Command::new(cmd).arg("--version").output() {
            if out.status.success() {
                // `py` (Windows launcher) 与 `python` 都返回可执行路径
                if let Ok(path) = std::process::Command::new(cmd)
                    .arg("-c")
                    .arg("import sys; print(sys.executable)")
                    .output()
                {
                    if path.status.success() {
                        let s = String::from_utf8_lossy(&path.stdout).trim().to_string();
                        if !s.is_empty() {
                            return Some(PathBuf::from(s));
                        }
                    }
                }
            }
        }
    }
    let fallbacks = [
        "C:\\Python313\\python.exe",
        "C:\\Python312\\python.exe",
        "C:\\Python311\\python.exe",
        "C:\\Python310\\python.exe",
    ];
    fallbacks.iter().map(PathBuf::from).find(|p| p.exists())
}

/// 解析 ClipNest 自带 rcr_ocr.py 路径。dev 模式优先找仓库脚本, release 从 resource_dir 找。
fn resolve_script_path(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidates = [
                parent.join(DEV_SCRIPT_REL),
                parent.join("..").join("..").join(DEV_SCRIPT_REL),
                parent.join("..").join("..").join("..").join(DEV_SCRIPT_REL),
                parent.join("..").join("..").join(TAURI_RESOURCE_SCRIPT_REL),
            ];
            for candidate in candidates {
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    if let Ok(dir) = app.path().resource_dir() {
        let candidate = dir.join(SUBDIR).join("rcr_ocr.py");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub struct RapidOcrEngine {
    python: Option<PathBuf>,
    script: Option<PathBuf>,
}

impl RapidOcrEngine {
    pub fn new() -> Self {
        Self {
            python: None,
            script: None,
        }
    }
    pub fn with_app(app: &AppHandle) -> Self {
        Self {
            python: find_python(),
            script: resolve_script_path(app),
        }
    }
    pub fn python_path(&self) -> Option<&Path> {
        self.python.as_deref()
    }
    pub fn script_path(&self) -> Option<&Path> {
        self.script.as_deref()
    }
}

impl Default for RapidOcrEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrEngine for RapidOcrEngine {
    fn name(&self) -> &'static str {
        "rapidocr"
    }

    fn available(&self) -> bool {
        self.python.is_some() && self.script.is_some()
    }

    fn recognize(&self, image_path: &Path) -> Result<OcrResult, OcrError> {
        let python = self.python.as_ref().ok_or_else(|| {
            OcrError::EngineUnavailable("未检测到 Python 3 (需要 Python 3.8+ 来运行 OCR)".into())
        })?;
        let script = self.script.as_ref().ok_or_else(|| {
            OcrError::EngineUnavailable("未找到 rcr_ocr.py;请重装 ClipNest".into())
        })?;
        if !image_path.exists() {
            return Err(OcrError::BadOutput(format!(
                "图片不存在: {}",
                image_path.display()
            )));
        }

        let started = Instant::now();
        let mut child = Command::new(python)
            .arg(script)
            .arg(image_path)
            .arg("--json")
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| OcrError::SpawnFailed(e.to_string()))?;

        // 用 try_wait 配合 sleep, 避免永久卡死
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
            let code = output.status.code().unwrap_or(-1);
            return Err(OcrError::NonZeroExit {
                code: Some(code),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        // stdout 末尾可能有 RapidOCR 残余日志行, 我们要找到 JSON 起止。
        // 安全做法: 取最后一行 { 开头的 JSON object.
        let buf = String::from_utf8_lossy(&output.stdout).into_owned();
        parse_rapid_output(&buf)
    }
}

/// 解析 stdout 找到最后一行 JSON `{...}`.
pub fn parse_rapid_output(stdout: &str) -> Result<OcrResult, OcrError> {
    let mut last_json: Option<&str> = None;
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            last_json = Some(trimmed);
        }
    }
    let json = last_json.ok_or_else(|| OcrError::BadOutput("未在输出中找到 JSON 行".into()))?;
    let value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| OcrError::BadOutput(format!("JSON 解析失败: {e}")))?;
    let text = value
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(OcrResult {
        text,
        source: "rapidocr".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pure_json() {
        let raw = r#"{"text": "hello", "elapsed_ms": 100, "lines": []}"#;
        let r = parse_rapid_output(raw).unwrap();
        assert_eq!(r.text, "hello");
        assert_eq!(r.source, "rapidocr");
    }

    #[test]
    fn parse_with_prefix_logs() {
        let raw = "INFO some log\nINFO another log\n{\"text\": \"world\", \"elapsed_ms\": 200}\n";
        let r = parse_rapid_output(raw).unwrap();
        assert_eq!(r.text, "world");
    }

    #[test]
    fn parse_picks_last_json_line() {
        let raw = "{\"text\": \"first\"}\nINFO\n{\"text\": \"second\"}\n";
        let r = parse_rapid_output(raw).unwrap();
        assert_eq!(r.text, "second");
    }

    #[test]
    fn parse_no_json_is_error() {
        let r = parse_rapid_output("no json here\n");
        assert!(matches!(r, Err(OcrError::BadOutput(_))));
    }

    #[test]
    fn parse_missing_text_field_defaults_to_empty() {
        let raw = r#"{"elapsed_ms": 100, "lines": []}"#;
        let r = parse_rapid_output(raw).unwrap();
        assert_eq!(r.text, "");
    }
}
