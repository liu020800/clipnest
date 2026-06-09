// Test backup command (mirrors commands::backup_database)
use rusqlite::{backup::Backup, Connection};
use std::path::Path;

fn main() {
    let appdata = std::env::var("APPDATA").unwrap();
    let data_dir = Path::new(&appdata).join("com.copyliusq.desktop");
    let db_path = data_dir.join("copyliusq.db");
    let backups_dir = data_dir.join("backups");

    println!("=== Backup test ===");
    println!("DB: {}", db_path.display());
    println!("Backups dir: {}", backups_dir.display());

    let initial_count: i64 = Connection::open(&db_path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
        .unwrap();
    println!("\n[1] Initial snippet count: {}", initial_count);

    // Run backup
    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let dest = backups_dir.join(format!("copyliusq-{}.db", ts));
    std::fs::create_dir_all(&backups_dir).unwrap();
    let mut dest_conn = Connection::open(&dest).unwrap();
    {
        let conn = Connection::open(&db_path).unwrap();
        let backup = Backup::new(&conn, &mut dest_conn).unwrap();
        backup
            .run_to_completion(100, std::time::Duration::from_millis(50), None)
            .unwrap();
    }
    let _ = dest_conn.close();
    println!("\n[2] Backup created: {}", dest.display());

    // Verify backup contains same data
    let restored = Connection::open(&dest).unwrap();
    let restored_count: i64 = restored
        .query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
        .unwrap();
    println!(
        "[3] Restored snippet count: {} (should match initial)",
        restored_count
    );

    if restored_count == initial_count {
        println!("\nPASS: backup integrity verified");
    } else {
        println!("\nFAIL: backup count mismatch");
    }

    // Verify the backup file is reasonable size
    let meta = std::fs::metadata(&dest).unwrap();
    println!("[4] Backup file size: {} bytes", meta.len());
}
