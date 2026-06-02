# Spec: Copyliusq — Personal Knowledge Capture System

## Objective

A Windows-native desktop app that captures clipboard content into a searchable, taggable personal knowledge base. The user copies something interesting (a command, a prompt, a config snippet, an idea), hits `Ctrl+Shift+S`, types a short title, and it's saved forever. Later they hit `Alt+Space` to search across everything.

**Target user:** Developers, writers, AI users who deal with many small pieces of text (commands, prompts, configs, snippets) and need a zero-friction capture-and-retrieve loop.

**Success criteria (MVP):**
- Copy text → `Ctrl+Shift+S` → type title → Enter → saved
- `Alt+Space` → fuzzy search across titles/content/tags → click/Enter to copy
- System tray with quick actions
- Deep dark theme, Raycast-like minimal UI

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri 2.x |
| Frontend | React 18 + TypeScript |
| Styling | Tailwind CSS |
| Local DB | SQLite (via rusqlite) |
| Full-text search | SQLite FTS5 |
| Global hotkeys | Tauri plugin (global-shortcut) |
| Clipboard monitor | Tauri plugin (clipboard) |
| Build target | Single exe (Windows) |

## Commands

```bash
# Development
cargo tauri dev          # Run in dev mode with hot reload

# Build
cargo tauri build        # Build release exe

# Frontend dev only (without Tauri)
cd src && npm run dev    # Vite dev server

# Type checking
cd src && npx tsc --noEmit

# Linting
cd src && npm run lint   # ESLint
```

## Project Structure

```
clipboard-ai/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs          # Tauri entry point
│   │   ├── clipboard.rs      # Clipboard monitor
│   │   ├── database.rs       # SQLite + FTS5 operations
│   │   ├── hotkey.rs         # Global hotkey registration
│   │   ├── search.rs         # FTS5 search logic
│   │   └── lib.rs            # Tauri command definitions
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── src/                     # React frontend
│   ├── components/
│   │   ├── SearchPanel.tsx   # Alt+Space search window
│   │   ├── SavePopup.tsx     # Ctrl+Shift+S save dialog
│   │   ├── SnippetCard.tsx   # Individual result card
│   │   └── TagBar.tsx        # Tag display/filter
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css            # Tailwind + dark theme
│
├── public/
├── package.json
├── tailwind.config.js
├── tsconfig.json
└── SPEC.md
```

## Code Style

- TypeScript, strict mode
- React functional components with hooks
- No class components
- Rust code following standard Rust style (rustfmt)
- Tauri commands are async Rust functions exposed via `#[tauri::command]`
- Frontend calls backend via `invoke()` from `@tauri-apps/api`
- CSS via Tailwind utility classes, no separate CSS files

## Database Schema

```sql
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    type TEXT,                      -- text/code/url
    tags TEXT,                      -- comma-separated
    source_app TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    pinned INTEGER DEFAULT 0
);

CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title,
    content,
    tags,
    content=snippets,
    content_rowid=id
);

-- Triggers to keep FTS in sync
CREATE TRIGGER snippets_ai AFTER INSERT ON snippets BEGIN
    INSERT INTO snippets_fts(rowid, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;
```

## Testing Strategy

- **Rust backend**: Unit tests inline with `#[cfg(test)]` — test database operations, search logic, tag parsing
- **Frontend**: Vitest + React Testing Library for component tests
- **Manual**: Verify clipboard capture, hotkey registration, system tray on Windows
- MVP does not require E2E tests

## Boundaries

- **Always:** Use FTS5 for search, dark theme default, keep UI under 600x500px, modular Rust code (one file per concern)
- **Ask first:** Adding a new Tauri plugin, changing the database schema, adding AI/Ollama integration, changing the global hotkey combinations
- **Never:** Store clipboard data without user confirmation (no auto-save), send data to external APIs, use Electron, commit secrets

## Success Criteria (MVP)

1. A user can copy text → press Ctrl+Shift+S → type a title → press Enter → snippet is saved
2. A user can press Alt+Space → type a query → see matching results → click or press Enter to copy
3. App runs in system tray, doesn't quit when window closes
4. Search matches on title, content, and tags with FTS5
5. Single exe build works on Windows 11
6. Deep dark theme, no light mode in MVP

## Open Questions

1. Project name: "copyliusq" — keep or rename?
2. Auto-save without title (just Ctrl+Shift+S with empty title saves with auto-generated title)?
3. Should snippets auto-tag based on content in MVP or wait for phase 2?
4. Do you already have Rust/Node toolchain installed or should I guide you through setup?
