use crate::database::Snippet;
use crate::settings::Settings;
use serde::Deserialize;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchArgs {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub filter_kind: Option<String>,
    #[serde(default)]
    pub filter_value: Option<String>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub pinned_only: Option<bool>,
}

fn is_valid_term(t: &str) -> bool {
    !t.is_empty() && t.chars().any(|c| !c.is_whitespace())
}

/// 把用户输入 term 转义成 FTS5 phrase 形式:
/// - 去除前导 `#`
/// - 内部 `"` 替换为 `""` (FTS5 phrase 内引号的转义规则)
/// - 移除 FTS5 语法元字符 `*` `:` `( )` `[ ]` 等,避免被解析为操作符
/// - 折叠所有空白(防止 term 跨行打断 phrase)
/// - 包裹成 `"term"*` 启用前缀匹配
fn sanitize_fts_term(t: &str) -> Option<String> {
    let cleaned: String = t
        .trim_start_matches('#')
        .chars()
        .filter(|c| !c.is_whitespace())
        .filter(|c| !matches!(c, '*' | ':' | '(' | ')' | '[' | ']'))
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    let escaped = cleaned.replace('"', "\"\"");
    Some(format!("\"{}\"*", escaped))
}

fn build_fts_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter_map(sanitize_fts_term)
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" AND "))
    }
}

fn like_fallback(
    db: &std::sync::MutexGuard<'_, crate::database::Database>,
    query: &str,
    limit: i64,
) -> Result<Vec<Snippet>, String> {
    let like = format!("%{}%", query.replace('%', ""));
    let mut stmt = db.conn_ref().prepare(
        "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned, pinyin, image_path, image_dim_w, image_dim_h, ocr_status
         FROM snippets
         WHERE title LIKE ?1 OR content LIKE ?1 OR tags LIKE ?1 OR pinyin LIKE ?1
         ORDER BY pinned DESC, created_at DESC
         LIMIT ?2",
    ).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![like, limit], |row| {
            crate::database::row_to_snippet(row)
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_snippets(
    state: tauri::State<crate::AppState>,
    query: String,
) -> Result<Vec<Snippet>, String> {
    let q = query.trim();

    if let Some(rest) = q.strip_prefix("#type:") {
        let args = SearchArgs {
            filter_kind: Some("type".to_string()),
            filter_value: Some(rest.to_string()),
            ..Default::default()
        };
        return list_snippets_inner(state, args);
    }
    if let Some(rest) = q.strip_prefix("#tag:") {
        let args = SearchArgs {
            filter_kind: Some("tag".to_string()),
            filter_value: Some(rest.to_string()),
            ..Default::default()
        };
        return list_snippets_inner(state, args);
    }
    if q == "#recent" {
        let args = SearchArgs {
            filter_kind: Some("recent".to_string()),
            ..Default::default()
        };
        return list_snippets_inner(state, args);
    }

    let args = SearchArgs {
        query,
        ..Default::default()
    };
    list_snippets_inner(state, args)
}

#[tauri::command]
pub fn list_snippets(
    state: tauri::State<crate::AppState>,
    args: SearchArgs,
) -> Result<Vec<Snippet>, String> {
    list_snippets_inner(state, args)
}

fn list_snippets_inner(
    state: tauri::State<crate::AppState>,
    args: SearchArgs,
) -> Result<Vec<Snippet>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let settings = Settings::load(&db);
    let limit = args.limit.unwrap_or(settings.search_limit).clamp(1, 500);
    let pinned_only = args.pinned_only.unwrap_or(false);
    let kind = args.filter_kind.as_deref().unwrap_or("");
    let value = args.filter_value.as_deref().unwrap_or("");

    if pinned_only || kind == "pinned" {
        return db.get_pinned(limit).map_err(|e| e.to_string());
    }
    if kind == "recent" {
        return db.get_recent(limit).map_err(|e| e.to_string());
    }
    if kind == "type" && !value.is_empty() {
        let mut stmt = db.conn_ref().prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned, pinyin, image_path, image_dim_w, image_dim_h, ocr_status
             FROM snippets
             WHERE type = ?1
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?2",
        ).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params![value, limit],
                crate::database::row_to_snippet,
            )
            .map_err(|e| e.to_string())?;
        return rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string());
    }
    if kind == "tag" && !value.is_empty() {
        let like = format!("%,{},%", value);
        let like_start = format!("{},%", value);
        let like_end = format!("%,{}", value);
        let like_exact = value.to_string();
        let mut stmt = db.conn_ref().prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned, pinyin, image_path, image_dim_w, image_dim_h, ocr_status
             FROM snippets
             WHERE tags = ?1 OR tags LIKE ?2 OR tags LIKE ?3 OR tags LIKE ?4
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?5",
        ).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params![like_exact, like, like_start, like_end, limit],
                crate::database::row_to_snippet,
            )
            .map_err(|e| e.to_string())?;
        return rows
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string());
    }

    // Free-text search via FTS5 (fall back to LIKE on error).
    let q = args.query.trim();
    if !is_valid_term(q) {
        return db.get_recent(limit).map_err(|e| e.to_string());
    }
    let fts = match build_fts_query(q) {
        Some(s) => s,
        None => return db.get_recent(limit).map_err(|e| e.to_string()),
    };
    let result = db.fts_search(&fts, limit);
    match result {
        Ok(v) => Ok(v),
        Err(_) => like_fallback(&db, q, limit),
    }
}
