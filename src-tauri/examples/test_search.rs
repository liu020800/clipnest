// One-off FTS5 + tag filter test
use rusqlite::Connection;

fn main() {
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| {
        let appdata = std::env::var("APPDATA").unwrap();
        format!("{}\\com.copyliusq.desktop\\copyliusq.db", appdata)
    });

    let conn = Connection::open(&db_path).expect("open db");

    println!("=== FTS5 搜索测试 ===\n");

    // Insert test snippets
    let tx = conn.unchecked_transaction().unwrap();
    tx.execute(
        "INSERT INTO snippets (title, content, pinyin, type, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "docker nginx",
            "docker run -d -p 80:80 nginx:latest",
            "docker nginx",
            "code",
            "docker,nginx,部署"
        ],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO snippets (title, content, pinyin, type, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "python script",
            "print('hello docker')",
            "python",
            "code",
            "python,docker"
        ],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO snippets (title, content, pinyin, type, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "github url",
            "https://github.com/test/repo",
            "github",
            "url",
            "网址,资料"
        ],
    )
    .unwrap();
    tx.commit().unwrap();

    // Test 1: FTS5 search for "docker"
    println!("[1] FTS5 search 'docker' (should find 2)");
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title, s.tags FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ?
             ORDER BY s.pinned DESC, f.rank",
            )
            .unwrap();
        let rows = stmt
            .query_map(["\"docker\"*"], |r| {
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

    // Test 2: tag filter (settings-seeded rules use LIKE on tags column)
    println!("\n[2] tag filter 'docker' (LIKE) (should find 2)");
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, tags FROM snippets
             WHERE tags = ?1 OR tags LIKE ?2 OR tags LIKE ?3 OR tags LIKE ?4
             ORDER BY pinned DESC, created_at DESC",
            )
            .unwrap();
        let tag = "docker";
        let like_middle = format!("%,{}%", tag);
        let like_start = format!("{},%", tag);
        let like_end = format!("%,{}", tag);
        let rows = stmt
            .query_map(
                rusqlite::params![tag, like_start, like_middle, like_end],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .unwrap();
        for r in rows {
            let (id, title, tags) = r.unwrap();
            println!("  #{} tags={:?} | {}", id, tags, title);
        }
    }

    // Test 3: type filter
    println!("\n[3] type filter 'code' (should find 2)");
    {
        let mut stmt = conn.prepare(
            "SELECT id, title, type FROM snippets WHERE type = ?1 ORDER BY pinned DESC, created_at DESC"
        ).unwrap();
        let rows = stmt
            .query_map(["code"], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .unwrap();
        for r in rows {
            let (id, title, t) = r.unwrap();
            println!("  #{} type={} | {}", id, t, title);
        }
    }

    // Test 4: pinned sort
    println!("\n[4] FTS5 search 'docker' with pin (first result should be pinned)");
    {
        // pin the docker nginx row
        conn.execute(
            "UPDATE snippets SET pinned = 1 WHERE title = 'docker nginx'",
            rusqlite::params![],
        )
        .unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title, s.pinned FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ?
             ORDER BY s.pinned DESC, f.rank",
            )
            .unwrap();
        let rows = stmt
            .query_map(["\"docker\"*"], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)? != 0,
                ))
            })
            .unwrap();
        for r in rows {
            let (id, title, pinned) = r.unwrap();
            println!("  #{} pinned={} | {}", id, pinned, title);
        }
    }

    // Test 5: update triggers FTS5
    println!("\n[5] Update title and verify FTS5 picks up new title (rename 'docker nginx' -> 'container orchestration')");
    conn.execute(
        "UPDATE snippets SET title = 'container orchestration' WHERE title = 'docker nginx'",
        rusqlite::params![],
    )
    .unwrap();
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ?
             ORDER BY s.pinned DESC, f.rank",
            )
            .unwrap();
        println!("  Search 'container' should now find the renamed row:");
        let rows = stmt
            .query_map(["\"container\"*"], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        for r in rows {
            let (id, title) = r.unwrap();
            println!("  #{} | {}", id, title);
        }
    }

    // Test 6: delete triggers FTS5
    println!("\n[6] Delete a row and verify FTS5 index removes it");
    conn.execute(
        "DELETE FROM snippets WHERE title = 'python script'",
        rusqlite::params![],
    )
    .unwrap();
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ?
             ORDER BY f.rank",
            )
            .unwrap();
        println!("  Search 'python' should now return 0:");
        let rows = stmt
            .query_map(["\"python\"*"], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  count: {}", count);
    }

    // Cleanup test data
    println!("\n[cleanup] Removing test rows");
    conn.execute(
        "DELETE FROM snippets WHERE title IN ('container orchestration', 'github url')",
        rusqlite::params![],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM snippets WHERE title = 'python script'",
        rusqlite::params![],
    )
    .unwrap();
    println!("done");
}
