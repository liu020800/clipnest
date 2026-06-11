use crate::database::Database;

fn seed_setting(db: &Database, key: &str, value: &str) {
    match db.get_setting(key) {
        Ok(Some(_)) => {}
        _ => {
            let _ = db.save_setting(key, value);
        }
    }
}

fn read_string(db: &Database, key: &str, default: &str) -> String {
    db.get_setting(key)
        .ok()
        .flatten()
        .unwrap_or_else(|| default.to_string())
}

fn read_bool(db: &Database, key: &str, default: bool) -> bool {
    match db.get_setting(key) {
        Ok(Some(v)) => v == "true",
        _ => default,
    }
}

fn read_i64(db: &Database, key: &str, default: i64) -> i64 {
    db.get_setting(key)
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default)
}

fn read_usize(db: &Database, key: &str, default: usize) -> usize {
    db.get_setting(key)
        .ok()
        .flatten()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Settings {
    pub search_limit: i64,
    pub search_debounce_ms: u64,
    pub auto_close_on_blur: bool,
    pub auto_close_delay_ms: u64,
    pub ai_enabled: bool,
    pub ollama_endpoint: String,
    pub ollama_model: String,
    pub ai_tag_fallback: String,
    pub auto_tag_on_capture: bool,
    pub capture_text_max_length: usize,
    pub markdown_export_pinned_only: bool,
    pub title_max_length: usize,
    pub capture_shortcut: String,
    pub capture_shortcut_alt: String,
    pub search_shortcut: String,
    pub screen_ocr_shortcut: String,
    pub autostart: bool,
    pub clipboard_history_enabled: bool,
    pub clipboard_history_max: i64,
}

impl Settings {
    pub fn load(db: &Database) -> Self {
        Self {
            search_limit: read_i64(db, "search_limit", 50).clamp(1, 500),
            search_debounce_ms: read_usize(db, "search_debounce_ms", 150) as u64,
            auto_close_on_blur: read_bool(db, "auto_close_on_blur", true),
            auto_close_delay_ms: read_usize(db, "auto_close_delay_ms", 150) as u64,
            ai_enabled: read_bool(db, "ai_enabled", false),
            ollama_endpoint: read_string(db, "ollama_endpoint", "http://localhost:11434"),
            ollama_model: read_string(db, "ollama_model", "qwen3:4b"),
            ai_tag_fallback: {
                let v = read_string(db, "ai_tag_fallback", "rules");
                if v == "none" {
                    "none".to_string()
                } else {
                    "rules".to_string()
                }
            },
            auto_tag_on_capture: read_bool(db, "auto_tag_on_capture", true),
            capture_text_max_length: read_usize(db, "capture_text_max_length", 50000),
            markdown_export_pinned_only: read_bool(db, "markdown_export_pinned_only", false),
            title_max_length: read_usize(db, "title_max_length", 10),
            capture_shortcut: read_string(db, "capture_shortcut", "Ctrl+Shift+S"),
            capture_shortcut_alt: read_string(db, "capture_shortcut_alt", "Alt+W"),
            search_shortcut: read_string(db, "search_shortcut", "Alt+Space"),
            screen_ocr_shortcut: read_string(db, "screen_ocr_shortcut", "Ctrl+Shift+O"),
            autostart: read_bool(db, "autostart", false),
            clipboard_history_enabled: read_bool(db, "clipboard_history_enabled", true),
            clipboard_history_max: read_i64(db, "clipboard_history_max", 500).clamp(50, 5000),
        }
    }
}

pub fn seed_defaults(db: &Database) {
    // autostart is set by setup() after reading the autostart plugin status
    seed_setting(db, "autostart", "false");
    seed_setting(db, "capture_shortcut", "Ctrl+Shift+S");
    seed_setting(db, "capture_shortcut_alt", "Alt+W");
    seed_setting(db, "search_shortcut", "Alt+Space");
    seed_setting(db, "screen_ocr_shortcut", "Ctrl+Shift+O");
    seed_setting(db, "clipboard_history_enabled", "true");
    seed_setting(db, "clipboard_history_max", "500");
    seed_setting(db, "search_limit", "50");
    seed_setting(db, "search_debounce_ms", "150");
    seed_setting(db, "auto_close_on_blur", "true");
    seed_setting(db, "auto_close_delay_ms", "150");
    seed_setting(db, "ai_enabled", "false");
    seed_setting(db, "ollama_endpoint", "http://localhost:11434");
    seed_setting(db, "ollama_model", "qwen3:4b");
    seed_setting(db, "ai_tag_fallback", "rules");
    seed_setting(db, "auto_tag_on_capture", "true");
    seed_setting(db, "capture_text_max_length", "50000");
    seed_setting(db, "markdown_export_pinned_only", "false");
    seed_setting(db, "title_max_length", "10");
    // Note: schema_version is NOT in the settings table; it lives in metadata.
    // normalize_v101_settings() in database.rs runs as a migration step.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_db() -> Database {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("copyliusq-settings-{}-{}.db", pid, n));
        let db = Database::new(path.to_str().unwrap()).expect("open temp db");
        db.init_settings().expect("init settings");
        db
    }

    #[test]
    fn seed_setting_does_not_overwrite() {
        let db = temp_db();
        seed_setting(&db, "k", "v1");
        assert_eq!(db.get_setting("k").unwrap().as_deref(), Some("v1"));
        seed_setting(&db, "k", "v2");
        assert_eq!(db.get_setting("k").unwrap().as_deref(), Some("v1"));
    }

    #[test]
    fn read_helpers_use_defaults() {
        let db = temp_db();
        assert_eq!(read_string(&db, "missing", "d"), "d");
        assert!(read_bool(&db, "missing", true));
        assert!(!read_bool(&db, "missing", false));
        assert_eq!(read_i64(&db, "missing", 42), 42);
        assert_eq!(read_usize(&db, "missing", 42), 42);
    }
}
