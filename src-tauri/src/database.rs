use pinyin::ToPinyin;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Snippet {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub pinyin: String,
    #[serde(rename = "type")]
    pub snippet_type: Option<String>,
    pub tags: Option<String>,
    pub source_app: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub pinned: bool,
    pub image_path: Option<String>,
    pub image_dim_w: Option<i64>,
    pub image_dim_h: Option<i64>,
    pub ocr_status: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=NORMAL;")?;
        let db = Database { conn };
        db.initialize()?;
        Ok(db)
    }

    pub fn conn_ref(&self) -> &Connection {
        &self.conn
    }

    pub fn backup_to(&self, dest: &std::path::Path) -> Result<(), String> {
        use rusqlite::backup::Backup;
        let mut dest_conn = Connection::open(dest).map_err(|e| e.to_string())?;
        let backup = Backup::new(&self.conn, &mut dest_conn).map_err(|e| e.to_string())?;
        backup
            .run_to_completion(100, std::time::Duration::from_millis(50), None)
            .map_err(|e| e.to_string())?;
        drop(backup);
        let _ = dest_conn.close();
        Ok(())
    }

    fn initialize(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;

        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS snippets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                pinyin TEXT DEFAULT '',
                type TEXT,
                tags TEXT,
                source_app TEXT,
                created_at DATETIME DEFAULT (datetime('now', 'localtime')),
                updated_at DATETIME DEFAULT (datetime('now', 'localtime')),
                pinned INTEGER DEFAULT 0
            );
            ",
        )?;

        // settings table (introduced in v1.0.0, kept for compatibility)
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;

        let current = self.get_schema_version().unwrap_or(0);
        eprintln!("[db] schema_version current = {current}");

        // Migration 1: add pinyin column for pre-v1.0 databases
        if current < 1 {
            self.ensure_column("snippets", "pinyin", "TEXT DEFAULT ''")?;
            self.set_schema_version(1)?;
        }
        // Migration 2: add image_path column (v1.1.0 introduced it, kept for forward compat)
        if current < 2 {
            self.ensure_column("snippets", "image_path", "TEXT")?;
            self.set_schema_version(2)?;
        }
        // Migration 3 (v1.0.1 final): normalize critical settings that drifted
        // from the v1.0.1 spec on older installs. seed_setting() only writes
        // when a key is missing, so users stuck with old default values need
        // an explicit bump. We use UPDATE (not INSERT) so user-customized
        // values are preserved.
        if current < 3 {
            self.normalize_v101_settings()?;
            self.set_schema_version(3)?;
        }
        // Migration 4 (v1.1): image capture + OCR. Add image_dim_w/h and
        // ocr_status columns. image_path was already added in v2.
        if current < 4 {
            self.ensure_column("snippets", "image_dim_w", "INTEGER")?;
            self.ensure_column("snippets", "image_dim_h", "INTEGER")?;
            self.ensure_column("snippets", "ocr_status", "TEXT")?;
            self.set_schema_version(4)?;
        }
        // Always run cleanup (idempotent): remove legacy garbage rows that
        // may have been written by older versions of seed_defaults.
        self.cleanup_legacy_settings_rows()?;

        self.conn.execute_batch(
            "
            CREATE VIRTUAL TABLE IF NOT EXISTS snippets_fts USING fts5(
                title,
                content,
                tags,
                pinyin,
                content=snippets,
                content_rowid=id
            );

            CREATE TRIGGER IF NOT EXISTS snippets_ai AFTER INSERT ON snippets BEGIN
                INSERT INTO snippets_fts(rowid, title, content, tags, pinyin)
                VALUES (new.id, new.title, new.content, new.tags, new.pinyin);
            END;

            CREATE TRIGGER IF NOT EXISTS snippets_ad AFTER DELETE ON snippets BEGIN
                INSERT INTO snippets_fts(snippets_fts, rowid, title, content, tags, pinyin)
                VALUES ('delete', old.id, old.title, old.content, old.tags, old.pinyin);
            END;

            CREATE TRIGGER IF NOT EXISTS snippets_au AFTER UPDATE ON snippets BEGIN
                INSERT INTO snippets_fts(snippets_fts, rowid, title, content, tags, pinyin)
                VALUES ('delete', old.id, old.title, old.content, old.tags, old.pinyin);
                INSERT INTO snippets_fts(rowid, title, content, tags, pinyin)
                VALUES (new.id, new.title, new.content, new.tags, new.pinyin);
            END;
            ",
        )?;
        Ok(())
    }

    fn get_schema_version(&self) -> Result<i64, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM metadata WHERE key = 'schema_version'")?;
        let mut rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(v)) => v.parse::<i64>().map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            }),
            _ => Ok(0),
        }
    }

    fn set_schema_version(&self, v: i64) -> Result<(), rusqlite::Error> {
        eprintln!("[db] migrating to schema_version {v}");
        self.conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES ('schema_version', ?1)",
            params![v.to_string()],
        )?;
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, decl: &str) -> Result<(), rusqlite::Error> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let cols: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(|r| r.ok())
            .collect();
        if cols.iter().any(|c| c.eq_ignore_ascii_case(column)) {
            eprintln!("[db] column {table}.{column} already exists, skip");
            return Ok(());
        }
        eprintln!("[db] ALTER TABLE {table} ADD COLUMN {column}");
        self.conn
            .execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl};"))?;
        Ok(())
    }

    pub fn insert_snippet(
        &self,
        title: &str,
        content: &str,
        tags: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        let snippet_type = detect_type(content);
        let pinyin = generate_pinyin(&format!("{} {} {}", title, content, tags.unwrap_or("")));
        self.conn.execute(
            "INSERT INTO snippets (title, content, pinyin, type, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![title, content, pinyin, snippet_type, tags],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn upsert_clipboard_history(
        &self,
        title: &str,
        content: &str,
        max_history: i64,
    ) -> Result<i64, rusqlite::Error> {
        if let Some(existing) = self.find_by_content(content)? {
            self.conn.execute(
                "UPDATE snippets
                 SET created_at = datetime('now', 'localtime'),
                     updated_at = datetime('now', 'localtime')
                 WHERE id = ?1",
                params![existing.id],
            )?;
            return Ok(existing.id);
        }

        let snippet_type = detect_type(content);
        let tags = "剪贴板";
        let pinyin = generate_pinyin(&format!("{title} {content} {tags}"));
        self.conn.execute(
            "INSERT INTO snippets (title, content, pinyin, type, tags, source_app)
             VALUES (?1, ?2, ?3, ?4, ?5, 'clipboard-history')",
            params![title, content, pinyin, snippet_type, tags],
        )?;
        let id = self.conn.last_insert_rowid();
        self.prune_clipboard_history(max_history)?;
        Ok(id)
    }

    fn prune_clipboard_history(&self, max_history: i64) -> Result<(), rusqlite::Error> {
        if max_history <= 0 {
            return Ok(());
        }
        self.conn.execute(
            "DELETE FROM snippets
             WHERE id IN (
                 SELECT id FROM snippets
                 WHERE source_app = 'clipboard-history' AND pinned = 0
                 ORDER BY created_at DESC
                 LIMIT -1 OFFSET ?1
             )",
            params![max_history],
        )?;
        Ok(())
    }

    pub fn insert_snippet_with_image(
        &self,
        title: &str,
        content: &str,
        tags: Option<&str>,
        image_path: &str,
        image_dim_w: Option<i64>,
        image_dim_h: Option<i64>,
        ocr_status: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        let snippet_type = "image";
        let pinyin = generate_pinyin(&format!("{} {} {}", title, content, tags.unwrap_or("")));
        self.conn.execute(
            "INSERT INTO snippets (title, content, pinyin, type, tags, image_path, image_dim_w, image_dim_h, ocr_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![title, content, pinyin, snippet_type, tags, image_path, image_dim_w, image_dim_h, ocr_status],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_ocr_status(
        &self,
        id: i64,
        status: &str,
        content: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        if let Some(c) = content {
            self.conn.execute(
                "UPDATE snippets SET content = ?1, ocr_status = ?2, updated_at = datetime('now', 'localtime') WHERE id = ?3",
                params![c, status, id],
            )?;
        } else {
            self.conn.execute(
                "UPDATE snippets SET ocr_status = ?1, updated_at = datetime('now', 'localtime') WHERE id = ?2",
                params![status, id],
            )?;
        }
        Ok(())
    }

    pub fn search(&self, query: &str, limit: i64) -> Result<Vec<Snippet>, rusqlite::Error> {
        if query.trim().is_empty() {
            return self.get_recent(limit);
        }
        let fts_query = build_fts_query(query);
        if fts_query.is_empty() {
            return Ok(Vec::new());
        }
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.title, s.content, s.type, s.tags,
                    s.source_app, s.created_at, s.updated_at, s.pinned, s.pinyin,
                    s.image_path, s.image_dim_w, s.image_dim_h, s.ocr_status
             FROM snippets s
             JOIN snippets_fts fts ON s.id = fts.rowid
             WHERE snippets_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts_query, limit], |row| row_to_snippet(row))?;
        rows.collect()
    }

    pub fn fts_search(&self, fts_query: &str, limit: i64) -> Result<Vec<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.title, s.content, s.type, s.tags,
                    s.source_app, s.created_at, s.updated_at, s.pinned, s.pinyin,
                    s.image_path, s.image_dim_w, s.image_dim_h, s.ocr_status
             FROM snippets s
             JOIN snippets_fts fts ON s.id = fts.rowid
             WHERE snippets_fts MATCH ?1
             ORDER BY s.pinned DESC, rank, s.created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts_query, limit], |row| row_to_snippet(row))?;
        rows.collect()
    }

    pub fn find_by_content(&self, content: &str) -> Result<Option<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned, pinyin, image_path, image_dim_w, image_dim_h, ocr_status
             FROM snippets
             WHERE content = ?1
             ORDER BY created_at DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![content], |row| row_to_snippet(row))?;
        match rows.next() {
            Some(Ok(s)) => Ok(Some(s)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    pub fn get_snippet_by_id(&self, id: i64) -> Result<Option<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned
                    , pinyin, image_path, image_dim_w, image_dim_h, ocr_status
                    FROM snippets WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| row_to_snippet(row))?;
        match rows.next() {
            Some(Ok(snippet)) => Ok(Some(snippet)),
            _ => Ok(None),
        }
    }

    pub fn get_pinned(&self, limit: i64) -> Result<Vec<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned
                    , pinyin, image_path, image_dim_w, image_dim_h, ocr_status
             FROM snippets
             WHERE pinned = 1
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| row_to_snippet(row))?;
        rows.collect()
    }

    pub fn get_recent(&self, limit: i64) -> Result<Vec<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned
                    , pinyin, image_path, image_dim_w, image_dim_h, ocr_status
             FROM snippets
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| row_to_snippet(row))?;
        rows.collect()
    }

    pub fn toggle_pin(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE snippets SET pinned = CASE WHEN pinned = 0 THEN 1 ELSE 0 END,
             updated_at = datetime('now', 'localtime') WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn delete_snippet(&self, id: i64) -> Result<(), rusqlite::Error> {
        self.conn
            .execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_snippet(
        &self,
        id: i64,
        title: Option<&str>,
        tags: Option<&str>,
        pinned: Option<bool>,
    ) -> Result<Option<Snippet>, rusqlite::Error> {
        use rusqlite::types::Value;
        let mut sets: Vec<String> = Vec::new();
        let mut bind_values: Vec<Value> = Vec::new();
        // 注意: 必须按 ?N 顺序追加 sets 和 bind_values。
        // ?1 始终是 id,后续从 ?2 开始按入参顺序编号。
        if let Some(t) = title {
            sets.push(format!("title = ?{}", bind_values.len() + 2));
            bind_values.push(Value::Text(t.to_string()));
        }
        if let Some(t) = tags {
            sets.push(format!("tags = ?{}", bind_values.len() + 2));
            bind_values.push(Value::Text(t.to_string()));
        }
        if let Some(b) = pinned {
            sets.push(format!("pinned = ?{}", bind_values.len() + 2));
            bind_values.push(Value::Integer(if b { 1 } else { 0 }));
        }
        if sets.is_empty() {
            return self.get_snippet_by_id(id);
        }
        sets.push("updated_at = datetime('now', 'localtime')".into());
        let sql = format!("UPDATE snippets SET {} WHERE id = ?1", sets.join(", "));

        // ?1 = id, 然后拼接各 set 对应的值。
        let mut all_params: Vec<&dyn rusqlite::ToSql> = vec![&id];
        for v in &bind_values {
            all_params.push(v);
        }
        self.conn.execute(&sql, all_params.as_slice())?;
        self.get_snippet_by_id(id)
    }

    pub fn get_all_snippets(&self) -> Result<Vec<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned, pinyin
                    , image_path, image_dim_w, image_dim_h, ocr_status
             FROM snippets ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| row_to_snippet(row))?;
        rows.collect()
    }

    pub fn init_settings(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        match rows.next() {
            Some(Ok(v)) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    pub fn get_all_settings(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    /// Normalize the critical v1.0.1 settings on existing installs.
    /// 历史实现: `INSERT OR REPLACE` 会覆盖用户已自定义的值(比如手动改过
    /// capture_shortcut 的用户在升级到 1.0.1 后会被强制改回 Ctrl+Shift+S)。
    /// 现在改为 `INSERT OR IGNORE`:
    /// - 已存在任何值的行: 保留用户选择
    /// - 不存在的行: 写入 v1.0.1 默认值
    /// 这意味着 seed_defaults() 与本函数互补,共同保证"未设置 → 有合理默认"。
    pub fn normalize_v101_settings(&self) -> Result<(), rusqlite::Error> {
        let tx = self.conn.unchecked_transaction()?;
        let pairs: &[(&str, &str)] = &[
            ("capture_shortcut", "Ctrl+Shift+S"),
            ("capture_shortcut_alt", "Alt+W"),
            ("search_shortcut", "Alt+Space"),
            ("screen_ocr_shortcut", "Ctrl+Shift+O"),
            ("title_max_length", "10"),
            ("ollama_model", "qwen3:4b"),
            ("ai_enabled", "false"),
            ("search_limit", "50"),
            ("search_debounce_ms", "150"),
            ("auto_close_on_blur", "true"),
            ("auto_close_delay_ms", "150"),
            ("auto_tag_on_capture", "true"),
            ("capture_text_max_length", "50000"),
            ("markdown_export_pinned_only", "false"),
            ("ollama_endpoint", "http://localhost:11434"),
            ("ai_tag_fallback", "rules"),
        ];
        for (k, v) in pairs {
            tx.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![k, v],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Idempotent cleanup: remove legacy garbage rows that older
    /// `seed_defaults` accidentally wrote to the `settings` table.
    pub fn cleanup_legacy_settings_rows(&self) -> Result<(), rusqlite::Error> {
        // schema_version 权威位置在 metadata 表
        self.conn
            .execute("DELETE FROM settings WHERE key = 'schema_version'", [])?;
        // image_path 列保留,UI 已禁用,不再需要 capture_images 设置
        self.conn
            .execute("DELETE FROM settings WHERE key = 'capture_images'", [])?;
        self.conn.execute(
            "DELETE FROM settings WHERE key = 'image_capture_enabled'",
            [],
        )?;
        Ok(())
    }

    pub fn list_tags(&self) -> Result<Vec<TagSummary>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT tags FROM snippets WHERE tags IS NOT NULL AND tags != ''")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for row in rows {
            let tags = row?;
            for t in tags.split(',') {
                let trimmed = t.trim();
                if !trimmed.is_empty() {
                    *counts.entry(trimmed.to_string()).or_insert(0) += 1;
                }
            }
        }
        let mut summaries: Vec<TagSummary> = counts
            .into_iter()
            .map(|(name, count)| TagSummary { name, count })
            .collect();
        summaries.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        Ok(summaries)
    }

    pub fn rename_tag(&self, old: &str, new: &str) -> Result<i64, rusqlite::Error> {
        if old == new {
            return Ok(0);
        }
        let tx = self.conn.unchecked_transaction()?;
        let mut stmt = tx.prepare(
            "SELECT id, tags FROM snippets WHERE tags LIKE ?1 OR tags LIKE ?2 OR tags = ?3",
        )?;
        let pat_start = format!("{}%", old);
        let pat_middle = format!("%,{}%", old);
        let rows: Vec<(i64, String)> = stmt
            .query_map(params![pat_start, pat_middle, old], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        let mut affected = 0i64;
        for (id, tags_str) in rows {
            let new_tags = rewrite_tags(&tags_str, old, new);
            if new_tags != tags_str {
                tx.execute("UPDATE snippets SET tags = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?2",
                    params![new_tags, id])?;
                affected += 1;
            }
        }
        tx.commit()?;
        Ok(affected)
    }

    pub fn delete_tag(&self, name: &str) -> Result<i64, rusqlite::Error> {
        let tx = self.conn.unchecked_transaction()?;
        let mut stmt = tx.prepare(
            "SELECT id, tags FROM snippets WHERE tags LIKE ?1 OR tags LIKE ?2 OR tags = ?3",
        )?;
        let pat_start = format!("{}%", name);
        let pat_middle = format!("%,{}%", name);
        let rows: Vec<(i64, String)> = stmt
            .query_map(params![pat_start, pat_middle, name], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        drop(stmt);
        let mut affected = 0i64;
        for (id, tags_str) in rows {
            let new_tags = remove_tag(&tags_str, name);
            if new_tags != tags_str {
                let new_val = if new_tags.is_empty() {
                    None
                } else {
                    Some(new_tags.as_str())
                };
                tx.execute("UPDATE snippets SET tags = ?1, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = ?2",
                    params![new_val, id])?;
                affected += 1;
            }
        }
        tx.commit()?;
        Ok(affected)
    }

    pub fn merge_tags(&self, from: &str, to: &str) -> Result<i64, rusqlite::Error> {
        if from == to {
            return Ok(0);
        }
        let renamed = self.rename_tag(from, to)?;
        Ok(renamed)
    }

    pub fn import_snippets(&self, items: &[Snippet]) -> Result<i64, rusqlite::Error> {
        let tx = self.conn.unchecked_transaction()?;
        let mut count = 0i64;
        for s in items {
            tx.execute(
                "INSERT INTO snippets (title, content, pinyin, type, tags, source_app, pinned, image_path, image_dim_w, image_dim_h, ocr_status, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, COALESCE(NULLIF(?12,''), strftime('%Y-%m-%dT%H:%M:%fZ','now')), COALESCE(NULLIF(?13,''), strftime('%Y-%m-%dT%H:%M:%fZ','now')))",
                params![
                    s.title,
                    s.content,
                    if s.pinyin.is_empty() { generate_pinyin(&format!("{} {}", s.title, s.content)) } else { s.pinyin.clone() },
                    s.snippet_type,
                    s.tags,
                    s.source_app,
                    s.pinned as i32,
                    s.image_path,
                    s.image_dim_w,
                    s.image_dim_h,
                    s.ocr_status,
                    s.created_at,
                    s.updated_at,
                ],
            )?;
            count += 1;
        }
        tx.commit()?;
        Ok(count)
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct TagSummary {
    pub name: String,
    pub count: i64,
}

fn rewrite_tags(tags_str: &str, old: &str, new: &str) -> String {
    let parts: Vec<&str> = tags_str.split(',').map(|s| s.trim()).collect();
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for p in parts {
        if p.is_empty() {
            continue;
        }
        let replaced = if p == old { new } else { p };
        if seen.insert(replaced.to_string()) {
            out.push(replaced.to_string());
        }
    }
    out.join(",")
}

fn remove_tag(tags_str: &str, name: &str) -> String {
    let parts: Vec<&str> = tags_str.split(',').map(|s| s.trim()).collect();
    let out: Vec<&str> = parts
        .into_iter()
        .filter(|p| *p != name && !p.is_empty())
        .collect();
    out.join(",")
}

pub fn row_to_snippet(row: &rusqlite::Row) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        snippet_type: row.get(3)?,
        tags: row.get(4)?,
        source_app: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        pinned: row.get::<_, i32>(8)? != 0,
        pinyin: row.get(9)?,
        image_path: row.get::<_, Option<String>>(10)?,
        image_dim_w: row.get::<_, Option<i64>>(11)?,
        image_dim_h: row.get::<_, Option<i64>>(12)?,
        ocr_status: row.get::<_, Option<String>>(13)?,
    })
}

fn detect_type(content: &str) -> &'static str {
    if content.starts_with("http://") || content.starts_with("https://") {
        "url"
    } else if content.contains('\n') || content.contains('\t') || content.contains("  ") {
        "code"
    } else {
        "text"
    }
}

fn generate_pinyin(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for c in text.chars() {
        if let Some(p) = c.to_pinyin() {
            result.push_str(p.plain());
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result.trim().to_string()
}

fn build_fts_query(query: &str) -> String {
    let terms = query
        .split_whitespace()
        .map(normalize_search_term)
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();

    if terms.is_empty() {
        return String::new();
    }

    terms
        .iter()
        .map(|term| format!("\"{}\"*", term))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn normalize_search_term(term: &str) -> String {
    let stripped = term.trim().trim_start_matches('#');
    stripped
        .chars()
        .filter(|c| !c.is_control() && !c.is_whitespace())
        .collect::<String>()
}

pub fn backup_database_file(
    src: &std::path::Path,
    backups_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    use rusqlite::backup::Backup;
    std::fs::create_dir_all(backups_dir).map_err(|e| e.to_string())?;
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let dest = backups_dir.join(format!("copyliusq-{}.db", ts));
    let mut dest_conn = Connection::open(&dest).map_err(|e| e.to_string())?;
    {
        let conn = Connection::open(src).map_err(|e| e.to_string())?;
        let backup = Backup::new(&conn, &mut dest_conn).map_err(|e| e.to_string())?;
        backup
            .run_to_completion(100, std::time::Duration::from_millis(50), None)
            .map_err(|e| e.to_string())?;
    }
    let _ = dest_conn.close();
    eprintln!("[db] backup created at {}", dest.display());
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_db() -> Database {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("copyliusq-dbtest-{}-{}.db", pid, n));
        let db = Database::new(path.to_str().unwrap()).expect("open temp db");
        db
    }

    #[test]
    fn empty_query_returns_recent_limited() {
        let db = temp_db();
        for i in 0..10 {
            db.insert_snippet(&format!("t{i}"), &format!("c{i}"), None)
                .unwrap();
        }
        let r1 = db.search("", 3).unwrap();
        assert_eq!(r1.len(), 3, "limit must cap result count");
    }

    #[test]
    fn search_respects_limit_for_keyword_query() {
        let db = temp_db();
        for i in 0..10 {
            db.insert_snippet(&format!("rust tip {i}"), "alpha content", Some("rust"))
                .unwrap();
        }
        let r = db.search("rust", 4).unwrap();
        assert_eq!(r.len(), 4, "FTS search must respect the limit parameter");
    }

    #[test]
    fn schema_version_is_set_after_init() {
        let _db = temp_db();
        let n = COUNTER.fetch_add(0, Ordering::SeqCst);
        let pid = std::process::id();
        // Use the same Database's metadata table via a fresh connection to the test DB path.
        let path = std::env::temp_dir().join(format!(
            "copyliusq-dbtest-{}-{}-{}.db",
            pid,
            n,
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _db2 = Database::new(path.to_str().unwrap()).unwrap();
        let conn = rusqlite::Connection::open(path.to_str().unwrap()).unwrap();
        let v: String = conn
            .query_row(
                "SELECT value FROM metadata WHERE key='schema_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(v.parse::<i64>().unwrap() >= 2);
    }

    #[test]
    fn find_by_content_finds_exact_match() {
        let db = temp_db();
        db.insert_snippet("hello", "unique marker 42", None)
            .unwrap();
        let found = db.find_by_content("unique marker 42").unwrap();
        assert!(found.is_some());
        let not_found = db.find_by_content("does not exist").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn clipboard_history_upsert_reuses_existing_content() {
        let db = temp_db();
        let first = db
            .upsert_clipboard_history("clip", "same clipboard text", 500)
            .unwrap();
        let second = db
            .upsert_clipboard_history("clip again", "same clipboard text", 500)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(db.get_all_snippets().unwrap().len(), 1);
    }

    #[test]
    fn clipboard_history_prunes_unpinned_auto_records() {
        let db = temp_db();
        for i in 0..4 {
            db.upsert_clipboard_history(&format!("clip {i}"), &format!("content {i}"), 2)
                .unwrap();
        }
        let auto_count = db
            .get_all_snippets()
            .unwrap()
            .into_iter()
            .filter(|s| s.source_app.as_deref() == Some("clipboard-history"))
            .count();
        assert_eq!(auto_count, 2);
    }

    #[test]
    fn fts_search_ranks_pinned_first() {
        let db = temp_db();
        db.insert_snippet("a", "docker compose example", Some("docker"))
            .unwrap();
        let id_b = db
            .insert_snippet("b", "another docker example", Some("docker"))
            .unwrap();
        db.toggle_pin(id_b).unwrap();
        let r = db.fts_search("\"docker\"*", 10).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r[0].pinned, "pinned item should be first");
    }

    #[test]
    fn image_snippet_round_trips_dimensions_and_ocr_status() {
        let db = temp_db();
        let id = db
            .insert_snippet_with_image(
                "截图",
                "提取的文字",
                Some("screenshot"),
                "originals/clip-1.png",
                Some(1920),
                Some(1080),
                Some("done"),
            )
            .unwrap();
        let loaded = db.get_snippet_by_id(id).unwrap().expect("exists");
        assert_eq!(loaded.image_path.as_deref(), Some("originals/clip-1.png"));
        assert_eq!(loaded.image_dim_w, Some(1920));
        assert_eq!(loaded.image_dim_h, Some(1080));
        assert_eq!(loaded.ocr_status.as_deref(), Some("done"));
        assert_eq!(loaded.snippet_type.as_deref(), Some("image"));
    }

    #[test]
    fn update_ocr_status_writes_text_and_status() {
        let db = temp_db();
        let id = db
            .insert_snippet_with_image(
                "img",
                "占位",
                None,
                "originals/clip-2.png",
                Some(10),
                Some(10),
                Some("pending"),
            )
            .unwrap();
        db.update_ocr_status(id, "done", Some("识别后的文字"))
            .unwrap();
        let loaded = db.get_snippet_by_id(id).unwrap().unwrap();
        assert_eq!(loaded.content, "识别后的文字");
        assert_eq!(loaded.ocr_status.as_deref(), Some("done"));
    }

    #[test]
    fn v1001_data_unchanged_after_v4_migration() {
        // v1.0.1 schema only had image_path; ensure v4 migration doesn't break
        // the round-trip of legacy fields.
        let db = temp_db();
        let id = db
            .insert_snippet("legacy", "legacy content", Some("legacy"))
            .unwrap();
        let loaded = db.get_snippet_by_id(id).unwrap().unwrap();
        assert_eq!(loaded.title, "legacy");
        assert_eq!(loaded.content, "legacy content");
        assert_eq!(loaded.tags.as_deref(), Some("legacy"));
        assert!(loaded.image_path.is_none());
        assert!(loaded.image_dim_w.is_none());
        assert!(loaded.image_dim_h.is_none());
        assert!(loaded.ocr_status.is_none());
    }
}
