// Test duplicate detection
use rusqlite::Connection;

fn main() {
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| {
        let appdata = std::env::var("APPDATA").unwrap();
        format!("{}\\com.copyliusq.desktop\\copyliusq.db", appdata)
    });

    let conn = Connection::open(&db_path).expect("open db");

    println!("=== 重复内容检测测试 ===\n");

    // Simulate commands::save_snippet_inner flow
    let test_content = "test unique content for dedup verification";

    // First insert
    println!("[1] First insert");
    let id1 = {
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO snippets (title, content, pinyin, type, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["first copy", test_content, "test", "text", "test"],
        )
        .unwrap();
        let id = tx.last_insert_rowid();
        tx.commit().unwrap();
        id
    };
    println!("  inserted id={}", id1);

    // find_by_content should find it
    println!("\n[2] find_by_content (should find id1)");
    {
        let mut stmt = conn.prepare("SELECT id, title FROM snippets WHERE content = ?1 ORDER BY created_at DESC LIMIT 1").unwrap();
        let mut rows = stmt
            .query_map([test_content], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        if let Some(r) = rows.next() {
            let (id, title) = r.unwrap();
            println!("  found id={} title={}", id, title);
        } else {
            println!("  ERROR: not found!");
        }
    }

    // Simulate DUPLICATE error format
    println!("\n[3] DUPLICATE error format");
    {
        let mut stmt = conn.prepare("SELECT id, title FROM snippets WHERE content = ?1 ORDER BY created_at DESC LIMIT 1").unwrap();
        let mut rows = stmt
            .query_map([test_content], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        if let Some(r) = rows.next() {
            let (id, title) = r.unwrap();
            // Match the format used in commands.rs
            let payload = serde_json::json!({ "id": id, "title": title });
            let err = format!("DUPLICATE::{}", payload);
            println!("  error string: {}", err);
            // Simulate parseDuplicateError on frontend side
            if let Some(rest) = err.strip_prefix("DUPLICATE::") {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(rest) {
                    println!("  parsed id={}, title={}", parsed["id"], parsed["title"]);
                }
            }
        }
    }

    // Cleanup
    println!("\n[cleanup] Removing test row");
    conn.execute("DELETE FROM snippets WHERE id = ?1", rusqlite::params![id1])
        .unwrap();
    println!("done");
}
