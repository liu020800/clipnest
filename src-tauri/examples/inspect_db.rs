// One-off DB inspection — placed in target/ to avoid affecting build
use rusqlite::Connection;

fn main() {
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| {
        let appdata = std::env::var("APPDATA").unwrap();
        format!("{}\\com.copyliusq.desktop\\copyliusq.db", appdata)
    });

    println!("=== DB: {} ===", db_path);
    let conn = Connection::open(&db_path).expect("open db");

    // Tables
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .unwrap();
    let tables: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    println!("\n[Tables] {:?}", tables);

    // Triggers
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='trigger' ORDER BY name")
        .unwrap();
    let triggers: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();
    println!("\n[Triggers] {:?}", triggers);

    // snippets_fts schema
    let mut stmt = conn
        .prepare("SELECT sql FROM sqlite_master WHERE name='snippets_fts'")
        .unwrap();
    if let Some(row) = stmt.query_row([], |r| r.get::<_, String>(0)).ok() {
        println!("\n[snippets_fts DDL]\n{}", row);
    }

    // Snippets count
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
        .unwrap();
    println!("\n[snippets count] {}", count);

    // Settings
    let mut stmt = conn
        .prepare("SELECT key, value FROM settings WHERE key != 'schema_version' ORDER BY key")
        .unwrap();
    println!("\n[Settings]");
    for r in stmt
        .query_map([], |r| {
            let k: String = r.get(0)?;
            let v: String = r.get(1)?;
            Ok((k, v))
        })
        .unwrap()
        .flatten()
    {
        let (k, v) = r;
        println!(
            "  {} = {}",
            k,
            if v.len() > 50 {
                format!("{}...", &v[..50])
            } else {
                v
            }
        );
    }

    // Metadata
    let mut stmt = conn
        .prepare("SELECT key, value FROM metadata ORDER BY key")
        .unwrap();
    println!("\n[Metadata]");
    for r in stmt
        .query_map([], |r| {
            let k: String = r.get(0)?;
            let v: String = r.get(1)?;
            Ok((k, v))
        })
        .unwrap()
        .flatten()
    {
        let (k, v) = r;
        println!("  {} = {}", k, v);
    }

    // Sample snippets (id, title, type, tags, created_at, pinned)
    println!("\n[Sample snippets (last 5)]");
    let mut stmt = conn.prepare("SELECT id, title, type, tags, pinned, datetime(created_at, 'localtime') FROM snippets ORDER BY id DESC LIMIT 5").unwrap();
    for r in stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .unwrap()
        .flatten()
    {
        let (id, title, typ, tags, pinned, dt) = r;
        let title_short = if title.chars().count() > 30 {
            format!("{}...", title.chars().take(30).collect::<String>())
        } else {
            title
        };
        println!(
            "  #{} [pinned={}] {} | type={:?} | tags={:?} | created={}",
            id, pinned, title_short, typ, tags, dt
        );
    }

    // Test FTS5 search
    println!("\n[FTS5 search 'docker' (top 3)]");
    {
        let mut stmt = conn.prepare("SELECT s.id, s.title FROM snippets s JOIN snippets_fts f ON s.id = f.rowid WHERE snippets_fts MATCH ? ORDER BY s.pinned DESC, f.rank LIMIT 3").unwrap();
        let rows = stmt
            .query_map(["docker"], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        for r in rows {
            let (id, title) = r.unwrap();
            println!("  #{} {}", id, title);
        }
    }

    // Test FTS5 search by tag
    println!("\n[FTS5 search by tag 'docker' (top 3)]");
    {
        let mut stmt = conn.prepare("SELECT id, title, tags FROM snippets WHERE tags LIKE '%docker%' ORDER BY id DESC LIMIT 3").unwrap();
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .unwrap();
        for r in rows {
            let (id, title, tags) = r.unwrap();
            println!("  #{} tags={:?} | {}", id, tags, title);
        }
    }
}
