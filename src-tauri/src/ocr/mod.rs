// OCR 抽象(v1.1)
// 一个 OcrEngine trait, 一个默认引擎 (RapidOCR via Python).
use std::time::Duration;

pub const OCR_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcrError {
    EngineUnavailable(String),
    SpawnFailed(String),
    Timeout,
    NonZeroExit { code: Option<i32>, stderr: String },
    BadOutput(String),
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OcrError::EngineUnavailable(m) => write!(f, "OCR 引擎不可用: {m}"),
            OcrError::SpawnFailed(m) => write!(f, "启动 OCR 失败: {m}"),
            OcrError::Timeout => write!(f, "OCR 超时 ({}s)", OCR_TIMEOUT.as_secs()),
            OcrError::NonZeroExit { code, stderr } => {
                write!(f, "OCR 退出码 {:?} stderr={}", code, stderr)
            }
            OcrError::BadOutput(m) => write!(f, "OCR 输出无法解析: {m}"),
        }
    }
}

impl std::error::Error for OcrError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrResult {
    pub text: String,
    pub source: String,
}

pub trait OcrEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn available(&self) -> bool;
    fn recognize(&self, image_path: &std::path::Path) -> Result<OcrResult, OcrError>;
}

pub mod rapidocr;
pub use rapidocr::RapidOcrEngine;
pub mod wechat;
pub use wechat::WechatOcrEngine;

/// 默认引擎: 始终是 RapidOCR (PaddleOCR-based, 离线, 不依赖本地微信).
pub fn default_engine() -> Box<dyn OcrEngine> {
    Box::new(RapidOcrEngine::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_engine_is_rapidocr() {
        let engine = default_engine();
        assert_eq!(engine.name(), "rapidocr");
    }
}
