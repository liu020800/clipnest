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

    fn initialize(&self) -> Result<(), rusqlite::Error> {
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

        // pinyin column already exists in CREATE TABLE above.
        // For databases from before pinyin was added, add it silently:
        let _ = self.conn.execute_batch(
            "ALTER TABLE snippets ADD COLUMN pinyin TEXT DEFAULT '';",
        );

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
                    s.source_app, s.created_at, s.updated_at, s.pinned, s.pinyin
             FROM snippets s
             JOIN snippets_fts fts ON s.id = fts.rowid
             WHERE snippets_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts_query, limit], |row| map_snippet(row))?;
        rows.collect()
    }

    pub fn get_snippet_by_id(&self, id: i64) -> Result<Option<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned
                    , pinyin FROM snippets WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| map_snippet(row))?;
        match rows.next() {
            Some(Ok(snippet)) => Ok(Some(snippet)),
            _ => Ok(None),
        }
    }

    pub fn get_pinned(&self, limit: i64) -> Result<Vec<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned
                    , pinyin
             FROM snippets
             WHERE pinned = 1
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| map_snippet(row))?;
        rows.collect()
    }

    pub fn get_recent(&self, limit: i64) -> Result<Vec<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned
                    , pinyin
             FROM snippets
             ORDER BY pinned DESC, created_at DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |row| map_snippet(row))?;
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
        self.conn.execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_snippet(
        &self,
        id: i64,
        title: Option<&str>,
        tags: Option<&str>,
        pinned: Option<bool>,
    ) -> Result<Option<Snippet>, rusqlite::Error> {
        let mut sets: Vec<String> = Vec::new();
        if title.is_some() { sets.push("title = ?2".into()); }
        if tags.is_some() { sets.push("tags = ?3".into()); }
        if pinned.is_some() { sets.push("pinned = ?4".into()); }
        if sets.is_empty() { return self.get_snippet_by_id(id); }
        sets.push("updated_at = datetime('now', 'localtime')".into());
        let sql = format!("UPDATE snippets SET {} WHERE id = ?1", sets.join(", "));
        let pin_val: Option<i32> = pinned.map(|b| if b { 1 } else { 0 });
        self.conn.execute(&sql, params![id, title, tags, pin_val])?;
        self.get_snippet_by_id(id)
    }

    pub fn get_all_snippets(&self) -> Result<Vec<Snippet>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, type, tags, source_app, created_at, updated_at, pinned, pinyin
             FROM snippets ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| map_snippet(row))?;
        rows.collect()
    }

    pub fn init_settings(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
        match rows.next() { Some(Ok(v)) => Ok(Some(v)), _ => Ok(None) }
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}

fn map_snippet(row: &rusqlite::Row) -> rusqlite::Result<Snippet> {
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
