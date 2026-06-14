use crate::ai;
use crate::database;
use crate::ocr::OcrEngine;
use crate::settings::Settings;
use crate::tags::auto_tag;
use std::path::{Component, Path, PathBuf};
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt;

fn resolve_app_image_path(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err("非法图片路径".into());
    }
    for component in rel_path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err("非法图片路径".into()),
        }
    }
    if !(rel.starts_with("originals/")
        || rel.starts_with("thumbs/")
        || rel.starts_with("originals\\")
        || rel.starts_with("thumbs\\"))
    {
        return Err("非法图片路径".into());
    }
    let candidate = root.join(rel_path);
    let root_canon = root.canonicalize().map_err(|e| e.to_string())?;
    let candidate_canon = candidate
        .canonicalize()
        .map_err(|_| "图片文件不存在".to_string())?;
    if !candidate_canon.starts_with(&root_canon) {
        return Err("非法图片路径".into());
    }
    Ok(candidate_canon)
}

#[tauri::command]
pub fn save_snippet(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
    title: String,
    content: String,
    tags: Option<String>,
) -> Result<i64, String> {
    save_snippet_inner(app, state, title, content, tags, false)
}

#[tauri::command]
pub fn save_snippet_force(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
    title: String,
    content: String,
    tags: Option<String>,
) -> Result<i64, String> {
    save_snippet_inner(app, state, title, content, tags, true)
}

fn save_snippet_inner(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
    title: String,
    content: String,
    tags: Option<String>,
    force: bool,
) -> Result<i64, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("内容为空,无法保存".to_string());
    }
    let title_trimmed = title.trim();
    if title_trimmed.is_empty() {
        return Err("标题不能为空".to_string());
    }
    if !force {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        if let Ok(Some(existing)) = db.find_user_saved_by_content(trimmed) {
            // 返回结构化错误,前端解析 DUPLICATE::{json}
            let payload = serde_json::json!({
                "id": existing.id,
                "title": existing.title,
            });
            return Err(format!("DUPLICATE::{}", payload));
        }
    }
    let final_tags = if let Some(t) = tags {
        Some(t)
    } else {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let enabled = Settings::load(&db).auto_tag_on_capture;
        drop(db);
        if enabled {
            auto_tag(trimmed, title_trimmed)
        } else {
            None
        }
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = db
        .save_user_snippet(title_trimmed, trimmed, final_tags.as_deref())
        .map_err(|e| e.to_string())?;
    drop(db);
    let _ = crate::tray::refresh_tray_menu(&app);
    Ok(id)
}

#[tauri::command]
pub fn delete_snippet(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
    id: i64,
) -> Result<(), String> {
    // 先取 image_path,删完库后再删磁盘文件,避免锁内做 IO。
    let image_path: Option<String> = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.get_snippet_by_id(id)
            .map_err(|e| e.to_string())?
            .and_then(|s| s.image_path)
    };
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.delete_snippet(id).map_err(|e| e.to_string())?;
    }
    // 物理删除: 当前图片存储保证 originals/xxx.png 与 thumbs/xxx.jpg 同 basename。
    if let Some(rel) = image_path {
        if !rel.is_empty() {
            if let Ok(data_dir) = app.path().app_data_dir() {
                let abs = data_dir.join(&rel);
                if let Err(e) = std::fs::remove_file(&abs) {
                    eprintln!("[delete_snippet] remove {abs:?} failed: {e}");
                }
                if rel.starts_with("originals/") || rel.starts_with("originals\\") {
                    let thumb_rel = rel.replacen("originals/", "thumbs/", 1).replacen(
                        "originals\\",
                        "thumbs\\",
                        1,
                    );
                    let thumb_abs = data_dir.join(thumb_rel).with_extension("jpg");
                    if let Err(e) = std::fs::remove_file(&thumb_abs) {
                        eprintln!("[delete_snippet] remove {thumb_abs:?} failed: {e}");
                    }
                }
            }
        }
    }
    let _ = crate::tray::refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub fn toggle_pin(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
    id: i64,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.toggle_pin(id).map_err(|e| e.to_string())?;
    drop(db);
    let _ = crate::tray::refresh_tray_menu(&app);
    Ok(())
}

#[tauri::command]
pub async fn auto_tag_ai(
    state: tauri::State<'_, crate::AppState>,
    title: String,
    content: String,
) -> Result<ai::AiTagResult, String> {
    let (enabled, endpoint, model, fallback) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let s = Settings::load(&db);
        (
            s.ai_enabled,
            s.ollama_endpoint,
            s.ollama_model,
            s.ai_tag_fallback,
        )
    };

    if !enabled {
        let rules_tags = parse_tags_csv(auto_tag(&content, &title));
        return match fallback.as_str() {
            "rules" => Ok(ai::AiTagResult {
                tags: rules_tags,
                summary: String::new(),
                source: "rules".to_string(),
            }),
            _ => Err("AI 标注已禁用".to_string()),
        };
    }

    match ai::ollama_tag(&content, &title, &endpoint, &model).await {
        Ok(mut r) => {
            // AI 摘要目前不写入数据库(避免 schema 变更),
            // 仅返回给前端展示,后端 source 标记 "ai"
            r.source = "ai".to_string();
            Ok(r)
        }
        Err(e) if fallback == "rules" => {
            let rules_tags = parse_tags_csv(auto_tag(&content, &title));
            if rules_tags.is_empty() {
                Err(e)
            } else {
                Ok(ai::AiTagResult {
                    tags: rules_tags,
                    summary: String::new(),
                    source: "rules".to_string(),
                })
            }
        }
        Err(e) => Err(e),
    }
}

fn parse_tags_csv(s: Option<String>) -> Vec<String> {
    s.unwrap_or_default()
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

#[tauri::command]
pub fn update_snippet(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
    id: i64,
    title: Option<String>,
    tags: Option<String>,
    pinned: Option<bool>,
) -> Result<Option<database::Snippet>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let result = db
        .update_snippet(id, title.as_deref(), tags.as_deref(), pinned)
        .map_err(|e| e.to_string())?;
    drop(db);
    let _ = crate::tray::refresh_tray_menu(&app);
    Ok(result)
}

fn detect_code_lang(content: &str) -> &'static str {
    let lower = content.to_lowercase();
    if lower.contains("fn ") || lower.contains("let ") || lower.contains("->") {
        "rust"
    } else if lower.contains("def ") || lower.contains("import ") || lower.contains("print(") {
        "python"
    } else if lower.contains("function ") || lower.contains("const ") || lower.contains("=>") {
        "javascript"
    } else if lower.contains("docker") || content.starts_with("#!/bin/") {
        "bash"
    } else if lower.contains("select ") || lower.contains("from ") || lower.contains("where ") {
        "sql"
    } else {
        ""
    }
}

#[tauri::command]
pub fn export_markdown(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
) -> Result<String, String> {
    let (snippets, pinned_only) = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let pinned_only = Settings::load(&db).markdown_export_pinned_only;
        let snippets = if pinned_only {
            db.get_pinned(100000).map_err(|e| e.to_string())?
        } else {
            db.get_all_snippets().map_err(|e| e.to_string())?
        };
        (snippets, pinned_only)
    };

    let now = chrono::Local::now();
    let filename = format!("clipnest-export-{}.md", now.format("%Y%m%d-%H%M%S"));
    let export_dir = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "无法获取文档目录".to_string())?
        .join("ClipNest");
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    let export_path = export_dir.join(&filename);
    let images_dir = export_dir.join("images");
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let mut md = String::new();
    md.push_str("# ClipNest 知识库导出\n\n");
    md.push_str(&format!(
        "导出时间: {}\n\n",
        now.format("%Y-%m-%d %H:%M:%S")
    ));
    if pinned_only {
        md.push_str("> 仅导出已固定收藏\n\n");
    }
    md.push_str(&format!("共 {} 条记录\n\n---\n\n", snippets.len()));

    for s in &snippets {
        md.push_str(&format!("## {}\n\n", s.title));
        if s.pinned {
            md.push_str("> 📌 已置顶\n\n");
        }
        md.push_str(&format!(
            "- **类型**: {}\n",
            s.snippet_type.as_deref().unwrap_or("text")
        ));
        md.push_str(&format!("- **创建**: {}\n", s.created_at));
        md.push_str(&format!("- **更新**: {}\n", s.updated_at));
        if let Some(tags) = &s.tags {
            if !tags.is_empty() {
                let tag_list: Vec<String> =
                    tags.split(',').map(|t| format!("#{}", t.trim())).collect();
                md.push_str(&format!("- **标签**: {}\n", tag_list.join(" ")));
            }
        }
        md.push_str("\n");

        // v1.1: 图片双路导出
        if let Some(image_rel) = &s.image_path {
            if !image_rel.is_empty() {
                std::fs::create_dir_all(&images_dir).map_err(|e| e.to_string())?;
                let src_abs = resolve_app_image_path(&data_dir, image_rel)?;
                if src_abs.exists() {
                    let file_name = src_abs
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "image.png".into());
                    let dst_abs = images_dir.join(&file_name);
                    if std::fs::copy(&src_abs, &dst_abs).is_ok() {
                        md.push_str(&format!("![](images/{})\n\n", file_name));
                    } else {
                        md.push_str(&format!("_图片导出失败: {}_\n\n", src_abs.display()));
                    }
                }
                // OCR 文本
                if !s.content.is_empty() {
                    let status = s.ocr_status.as_deref().unwrap_or("");
                    let label = match status {
                        "done" => "提取文字",
                        "skipped" => "OCR 已跳过",
                        "failed" => "OCR 失败",
                        _ => "内容",
                    };
                    md.push_str(&format!("> **{}:**\n\n", label));
                    let lang = match s.snippet_type.as_deref() {
                        Some("code") => detect_code_lang(&s.content),
                        _ => "",
                    };
                    md.push_str(&format!("```{}\n{}\n```\n\n", lang, s.content));
                }
                md.push_str("---\n\n");
                continue;
            }
        }

        // 非图片: 走原路径
        let lang = match s.snippet_type.as_deref() {
            Some("url") => "",
            Some("code") => detect_code_lang(&s.content),
            _ => "",
        };
        md.push_str(&format!("```{}\n{}\n```\n\n", lang, s.content));
        md.push_str("---\n\n");
    }

    std::fs::write(&export_path, md).map_err(|e| e.to_string())?;
    Ok(export_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_settings(app: tauri::AppHandle) {
    crate::show_window(&app, "settings");
}

#[tauri::command]
pub fn open_capture(app: tauri::AppHandle) {
    crate::show_window(&app, "capture");
}

#[tauri::command]
pub fn hide_window(app: tauri::AppHandle, label: String) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(&label) {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn get_autostart(state: tauri::State<crate::AppState>) -> Result<bool, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_setting("autostart")
        .map_err(|e| e.to_string())
        .map(|v| v.as_deref() == Some("true"))
}

#[tauri::command]
pub fn set_autostart(
    app: tauri::AppHandle,
    state: tauri::State<crate::AppState>,
    enable: bool,
) -> Result<(), String> {
    let manager = app.autolaunch();
    if enable {
        manager.enable().map_err(|e| e.to_string())?;
    } else {
        manager.disable().map_err(|e| e.to_string())?;
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_setting("autostart", if enable { "true" } else { "false" })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_setting(
    state: tauri::State<crate::AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.save_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_all_settings(
    state: tauri::State<crate::AppState>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.get_all_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_db_path(app: tauri::AppHandle) -> Result<String, String> {
    let p = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(p.join("copyliusq.db").to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_db_dir(app: tauri::AppHandle) -> Result<String, String> {
    let p = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = p.to_string_lossy().to_string();
    open_in_explorer(PathBuf::from(&p));
    Ok(dir)
}

fn open_in_explorer(p: PathBuf) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&p).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&p).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
    }
}

#[tauri::command]
pub fn backup_database(app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let db_path = data_dir.join("copyliusq.db");
    let backups_dir = data_dir.join("backups");
    let dest = database::backup_database_file(&db_path, &backups_dir)?;
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn list_tags(
    state: tauri::State<crate::AppState>,
) -> Result<Vec<database::TagSummary>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_tags().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rename_tag(
    state: tauri::State<crate::AppState>,
    old: String,
    new: String,
) -> Result<i64, String> {
    if old.trim().is_empty() || new.trim().is_empty() {
        return Err("标签名不能为空".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.rename_tag(old.trim(), new.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_tag(state: tauri::State<crate::AppState>, name: String) -> Result<i64, String> {
    if name.trim().is_empty() {
        return Err("标签名不能为空".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_tag(name.trim()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn merge_tags(
    state: tauri::State<crate::AppState>,
    from: String,
    to: String,
) -> Result<i64, String> {
    if from.trim().is_empty() || to.trim().is_empty() {
        return Err("标签名不能为空".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.merge_tags(from.trim(), to.trim())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_json(state: tauri::State<crate::AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let snippets = db.get_all_snippets().map_err(|e| e.to_string())?;
    drop(db);

    let now = chrono::Local::now();
    let filename = format!("clipnest-export-{}.json", now.format("%Y%m%d-%H%M%S"));
    let export_dir = dirs::document_dir()
        .or_else(dirs::home_dir)
        .ok_or_else(|| "无法获取文档目录".to_string())?
        .join("ClipNest");
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;
    let export_path = export_dir.join(&filename);

    let payload = serde_json::json!({
        "version": 1,
        "exported_at": now.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "count": snippets.len(),
        "snippets": snippets,
    });
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    std::fs::write(&export_path, json).map_err(|e| e.to_string())?;
    Ok(export_path.to_string_lossy().to_string())
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub imported: i64,
    pub path: String,
}

#[tauri::command]
pub fn import_json(
    state: tauri::State<crate::AppState>,
    path: String,
) -> Result<ImportResult, String> {
    if path.trim().is_empty() {
        return Err("未选择文件".to_string());
    }
    let import_path = std::path::PathBuf::from(path.trim());
    if !import_path.exists() {
        return Err(format!("文件不存在: {}", import_path.display()));
    }
    let text = std::fs::read_to_string(&import_path).map_err(|e| e.to_string())?;
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let items: Vec<database::Snippet> = match parsed.get("snippets") {
        Some(serde_json::Value::Array(arr)) => {
            let mut out = Vec::with_capacity(arr.len());
            for v in arr {
                match serde_json::from_value::<database::Snippet>(v.clone()) {
                    Ok(s) => out.push(s),
                    Err(e) => return Err(format!("JSON 内容解析失败: {e}")),
                }
            }
            out
        }
        _ => return Err("JSON 格式错误,缺少 snippets 数组".to_string()),
    };
    if items.is_empty() {
        return Err("JSON 中没有任何片段可导入".to_string());
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let count = db.import_snippets(&items).map_err(|e| e.to_string())?;
    Ok(ImportResult {
        imported: count,
        path: import_path.to_string_lossy().to_string(),
    })
}

// === v1.1: screen-region OCR ===

#[derive(serde::Serialize)]
pub struct OcrDoneInfo {
    pub text: String,
    pub source: String,
}

#[derive(serde::Deserialize)]
pub struct ScreenOcrRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[tauri::command]
pub fn capture_screen_ocr_region(
    app: tauri::AppHandle,
    region: ScreenOcrRegion,
) -> Result<OcrDoneInfo, String> {
    if region.width < 8 || region.height < 8 {
        return Err("框选区域过小".to_string());
    }
    if region.width > 8000 || region.height > 8000 {
        return Err("框选区域过大".to_string());
    }

    if let Some(window) = app.get_webview_window("screen_ocr") {
        let _ = window.hide();
    }
    std::thread::sleep(std::time::Duration::from_millis(220));

    let screen = screenshots::Screen::from_point(region.x, region.y).map_err(|e| e.to_string())?;
    let local_x = region.x - screen.display_info.x;
    let local_y = region.y - screen.display_info.y;
    let image = screen
        .capture_area(local_x, local_y, region.width, region.height)
        .map_err(|e| e.to_string())?;

    let tmp_dir = app
        .path()
        .app_cache_dir()
        .or_else(|_| app.path().app_data_dir())
        .map_err(|e| e.to_string())?
        .join("screen-ocr");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;
    let image_path = tmp_dir.join(format!(
        "region-{}.png",
        chrono::Utc::now().format("%Y%m%d%H%M%S%f")
    ));
    image.save(&image_path).map_err(|e| e.to_string())?;
    let latest_path = tmp_dir.join("latest-region.png");
    let _ = std::fs::copy(&image_path, &latest_path);

    let rapid = crate::ocr::RapidOcrEngine::with_app(&app);
    let wechat = crate::ocr::WechatOcrEngine::with_app(&app);
    let result = match rapid.recognize(&image_path).or_else(|_| wechat.recognize(&image_path)) {
        Ok(r) => Ok(OcrDoneInfo {
            text: r.text,
            source: r.source,
        }),
        Err(e) => Err(e.to_string()),
    };
    let _ = std::fs::remove_file(&image_path);
    result
}

#[tauri::command]
pub fn set_pending_capture_text(
    state: tauri::State<crate::AppState>,
    text: String,
) -> Result<(), String> {
    let mut pending = state
        .pending_capture_text
        .lock()
        .map_err(|e| e.to_string())?;
    *pending = Some(text);
    Ok(())
}

#[tauri::command]
pub fn take_pending_capture_text(state: tauri::State<crate::AppState>) -> Result<String, String> {
    let mut pending = state
        .pending_capture_text
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(pending.take().unwrap_or_default())
}

#[derive(serde::Serialize)]
pub struct OcrCapability {
    pub engine: String,
    pub available: bool,
    pub python_detected: bool,
    pub python_path: Option<String>,
    pub script_bundled: bool,
    pub message: Option<String>,
}

/// 给前端用来判断 "提取文字" 按钮是否启用。
#[tauri::command]
pub fn get_ocr_capability(app: tauri::AppHandle) -> OcrCapability {
    let wechat = crate::ocr::WechatOcrEngine::with_app(&app);
    let rapid = crate::ocr::RapidOcrEngine::with_app(&app);
    let python_detected = rapid.python_path().is_some();
    let script_bundled = rapid.script_path().is_some();
    let rapid_available = python_detected && script_bundled;
    let wechat_available = wechat.available();
    let available = wechat_available || rapid_available;
    let message = if !available {
        let mut parts = Vec::new();
        parts.push("未找到可用的 WeChatOCR 引擎文件".to_string());
        if !python_detected {
            parts.push("未检测到 Python 3 (需要 3.8+)".to_string());
        }
        if !script_bundled {
            parts.push("未找到 rcr_ocr.py".to_string());
        }
        Some(parts.join("; "))
    } else {
        None
    };
    OcrCapability {
        engine: if rapid_available {
            "rapidocr"
        } else {
            "wechatocr"
        }
        .into(),
        available,
        python_detected,
        python_path: rapid
            .python_path()
            .map(|p| p.to_string_lossy().to_string()),
        script_bundled,
        message,
    }
}

/// 把 AppData 相对路径转成 Webview 可访问的 asset URL。
/// 实际访问用 tauri://localhost/IMAGE?path=rel, 这里先返回原始 rel 字符串,
/// 前端用 <img src={`tauri://localhost/IMAGE?path=${rel}`}> 显示。
/// 简化: 前端用 convertFileSrc(绝对路径)。
#[tauri::command]
pub fn resolve_image_path(app: tauri::AppHandle, rel: String) -> Result<String, String> {
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let abs = resolve_app_image_path(&root, &rel)?;
    Ok(abs.to_string_lossy().to_string())
}
