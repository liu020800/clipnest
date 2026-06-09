// End-to-end insert + search test
use rusqlite::Connection;

fn main() {
    let db_path = std::env::var("DB_PATH").unwrap_or_else(|_| {
        let appdata = std::env::var("APPDATA").unwrap();
        format!("{}\\com.copyliusq.desktop\\copyliusq.db", appdata)
    });

    let conn = Connection::open(&db_path).expect("open db");
    println!("=== End-to-end search test ===\n");

    // Cleanup any prior test data
    conn.execute("DELETE FROM snippets WHERE title LIKE 'TEST_%'", [])
        .unwrap();

    // Simulate the full save flow
    let test_content = "docker run -d -p 80:80 nginx:latest\n# 测试内容";
    let test_title = "TEST_docker_nginx";
    let pinyin = "test";
    let snippet_type = "code";
    let tags = "docker,nginx,部署,TEST";

    // 1. insert
    let id: i64 = {
        let tx = conn.unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO snippets (title, content, pinyin, type, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![test_title, test_content, pinyin, snippet_type, tags],
        )
        .unwrap();
        let id = tx.last_insert_rowid();
        tx.commit().unwrap();
        id
    };
    println!("[1] INSERT id={}", id);

    // 2. FTS5 search by content
    println!("\n[2] FTS5 search 'docker' (expect 1 result)");
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ?
             AND s.id = ?2",
            )
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params!["\"docker\"*", id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  found: {}", count);
    }

    // 3. FTS5 search by Chinese 部署
    println!("\n[3] FTS5 search '部署' (expect 1 result)");
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ?
             AND s.id = ?2",
            )
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params!["\"部署\"*", id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  found: {}", count);
    }

    // 4. FTS5 search by pinyin
    println!("\n[4] FTS5 search pinyin 'test' (expect 1 result)");
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ?
             AND s.id = ?2",
            )
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params!["\"test\"*", id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  found: {}", count);
    }

    // 5. tag filter via LIKE
    println!("\n[5] tag filter 'docker' (expect 1 result)");
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, title, tags FROM snippets
             WHERE tags = ?1 OR tags LIKE ?2 OR tags LIKE ?3 OR tags LIKE ?4
             AND id = ?5",
            )
            .unwrap();
        let tag = "docker";
        let rows = stmt
            .query_map(
                rusqlite::params![
                    tag,
                    format!("{},%", tag),
                    format!("%,{}%", tag),
                    format!("%,{}", tag),
                    id
                ],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .unwrap();
        let count = rows.count();
        println!("  found: {}", count);
    }

    // 6. update title to verify trigger
    println!("\n[6] UPDATE title to 'TEST_docker_renamed'");
    conn.execute(
        "UPDATE snippets SET title = 'TEST_docker_renamed' WHERE id = ?1",
        rusqlite::params![id],
    )
    .unwrap();
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ? AND s.id = ?2",
            )
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params!["\"renamed\"*", id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  search 'renamed' found: {}", count);
    }

    // 7. update tags
    println!("\n[7] UPDATE tags to 'newtag1,newtag2'");
    conn.execute(
        "UPDATE snippets SET tags = 'newtag1,newtag2' WHERE id = ?1",
        rusqlite::params![id],
    )
    .unwrap();
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.tags FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ? AND s.id = ?2",
            )
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params!["\"newtag1\"*", id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  search 'newtag1' found: {}", count);
    }

    // 8. duplicate detection
    println!("\n[8] find_by_content (expect 1 result)");
    {
        let mut stmt = conn
            .prepare("SELECT id, title FROM snippets WHERE content = ?1 AND id = ?2")
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params![test_content, id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  found: {}", count);
    }

    // 9. delete
    println!("\n[9] DELETE row, verify FTS5 cleans up");
    conn.execute("DELETE FROM snippets WHERE id = ?1", rusqlite::params![id])
        .unwrap();
    {
        let mut stmt = conn
            .prepare(
                "SELECT s.id, s.title FROM snippets s
             JOIN snippets_fts f ON s.id = f.rowid
             WHERE snippets_fts MATCH ? AND s.id = ?2",
            )
            .unwrap();
        let rows = stmt
            .query_map(rusqlite::params!["\"renamed\"*", id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap();
        let count = rows.count();
        println!("  search 'renamed' after delete found: {}", count);
    }

    // 10. cleanup any leftover TEST_
    conn.execute("DELETE FROM snippets WHERE title LIKE 'TEST_%'", [])
        .unwrap();

    println!("\n=== All search/trigger tests done ===");
}
