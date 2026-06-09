# 数据库迁移与备份

## 当前 schema_version

| 版本 | 变更 | 引入版本 |
|------|------|----------|
| 1 | 初始 schema,`snippets` / `settings` 表,`snippets_fts` 虚表 | v1.0.0 |
| 2 | 新增 `metadata` 表(记录 `schema_version`)、`snippets.pinyin`、`snippets.image_path` 字段 | v1.0.1 |

## 当前 v1.0.1 表结构

```sql
-- 主表
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    pinyin TEXT DEFAULT '',
    type TEXT,                       -- 'text' | 'code' | 'url' | 'prompt'
    tags TEXT,                       -- 逗号分隔
    source_app TEXT,
    created_at DATETIME DEFAULT (datetime('now','localtime')),
    updated_at DATETIME DEFAULT (datetime('now','localtime')),
    pinned INTEGER DEFAULT 0,
    image_path TEXT                  -- 保留字段,UI 已禁用
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- FTS5 虚表
CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title, content, tags, pinyin,
    content=snippets,
    content_rowid=id
);

-- 触发器保持 FTS 同步
CREATE TRIGGER snippets_ai AFTER INSERT ON snippets BEGIN
    INSERT INTO snippets_fts(rowid, title, content, tags, pinyin)
    VALUES (new.id, new.title, new.content, new.tags, new.pinyin);
END;

CREATE TRIGGER snippets_ad AFTER DELETE ON snippets BEGIN
    INSERT INTO snippets_fts(snippets_fts, rowid, title, content, tags, pinyin)
    VALUES ('delete', old.id, old.title, old.content, old.tags, old.pinyin);
END;

CREATE TRIGGER snippets_au AFTER UPDATE ON snippets BEGIN
    INSERT INTO snippets_fts(snippets_fts, rowid, title, content, tags, pinyin)
    VALUES ('delete', old.id, old.title, old.content, old.tags, old.pinyin);
    INSERT INTO snippets_fts(rowid, title, content, tags, pinyin)
    VALUES (new.id, new.title, new.content, new.tags, new.pinyin);
END;
```

## 启动时备份

每次启动时,如果数据库已存在,自动备份一份到 `backups/`:

```
%APPDATA%\com.copyliusq.desktop\
├── copyliusq.db                 -- 当前数据库
└── backups/
    ├── copyliusq-20260604-093137.db
    ├── copyliusq-20260604-093230.db
    └── ...
```

每个备份文件按时间戳命名。**不会自动清理旧备份** — 用户可在文件管理器手动删除。

## 手动备份

设置页 → 数据 → "立即备份" 按钮,可即时创建带时间戳的备份。

## 迁移步骤

所有迁移在 `database.rs` 的 `migrate()` 中按版本顺序执行。每个迁移使用 `ensure_column()` 检查目标列是否已存在,保证**幂等**。

```rust
fn migrate(&self) -> rusqlite::Result<()> {
    let version = self.get_schema_version().unwrap_or(0);
    if version < 1 {
        // 初始 schema
    }
    if version < 2 {
        // 添加 pinyin, image_path, metadata 表
        self.ensure_column("snippets", "pinyin", "TEXT DEFAULT ''")?;
        self.ensure_column("snippets", "image_path", "TEXT")?;
    }
    // 写入新版本
    self.set_schema_version(2)?;
    Ok(())
}
```

## 恢复

1. 关闭 ClipNest
2. 找到最近的备份 `copyliusq-YYYYMMDD-HHMMSS.db`
3. 复制覆盖当前的 `copyliusq.db`
4. 重启 ClipNest

## 故障排除

- **"FTS5 错误"**: SQLite 缺少 FTS5 支持。ClipNest 使用 rusqlite 的 `bundled` feature,自带 SQLite 编译,应当不存在此问题。
- **"schema version not found"**: 数据库结构损坏或人为修改。备份后删除 `copyliusq.db` 让程序重建,数据会丢失。
- **启动时 panic**: 检查 stderr 是否有 `[db] migration failed` 日志。
