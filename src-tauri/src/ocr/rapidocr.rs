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
        let mut version_cmd = std::process::Command::new(cmd);
        version_cmd.arg("--version");
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            version_cmd.creation_flags(CREATE_NO_WINDOW);
        }
        if let Ok(out) = version_cmd.output() {
            if out.status.success() {
                // `py` (Windows launcher) 与 `python` 都返回可执行路径
                let mut path_cmd = std::process::Command::new(cmd);
                path_cmd
                    .arg("-c")
                    .arg("import sys; print(sys.executable)");
                #[cfg(target_os = "windows")]
                {
                    use std::os::windows::process::CommandExt;
                    const CREATE_NO_WINDOW: u32 = 0x08000000;
                    path_cmd.creation_flags(CREATE_NO_WINDOW);
                }
                if let Ok(path) = path_cmd.output() {
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
        // Tauri v2 打包时会保留 bundle.resources 里配置的相对路径结构 ("resources/..."),
        // 所以真实路径是 resource_dir()/resources/rapidocr/rcr_ocr.py。这里两种都试,
        // 兼容不同 Tauri 版本/打包器对 resource_dir 根的差异。
        let candidates = [
            dir.join("resources").join(SUBDIR).join("rcr_ocr.py"),
            dir.join(SUBDIR).join("rcr_ocr.py"),
        ];
        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate);
            }
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
        let mut cmd = Command::new(python);
        cmd.arg(script)
            .arg(image_path)
            .arg("--json")
            .env("PYTHONIOENCODING", "utf-8")
            .env("PYTHONUTF8", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| OcrError::SpawnFailed(e.to_string()))?;

        // Windows 上 pipe buffer 约 64KB,RapidOCR/PaddleOCR 初始化日志或大图识别结果一旦填满
        // 就会让子进程在 write 上阻塞,主线程的 try_wait 永远拿不到退出 → 必然走到 30s 超时。
        // 必须在 spawn 之后立刻起后台线程 drain stdout/stderr,主线程才能安全地 try_wait + timeout。
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
                // join 是为了释放 pipe 句柄,避免线程泄漏。
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
            let code = exit_status.code().unwrap_or(-1);
            return Err(OcrError::NonZeroExit {
                code: Some(code),
                stderr: String::from_utf8_lossy(&stderr_bytes).to_string(),
            });
        }
        // stdout 末尾可能有 RapidOCR 残余日志行, 我们要找到 JSON 起止。
        // 安全做法: 取最后一行 { 开头的 JSON object.
        let buf = String::from_utf8_lossy(&stdout_bytes).into_owned();
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
