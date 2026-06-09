# Spec: ClipNest v1.0.1 — Personal Knowledge Capture System

## Objective

A Windows-native desktop app that captures clipboard content into a searchable, taggable personal knowledge base. The user copies something interesting (a command, a prompt, a config snippet, an idea), hits `Ctrl+Shift+S`, types a short title (default 10 chars), and it's saved forever. Later they hit `Alt+Space` to search across everything.

**Target user:** Developers, writers, AI users who deal with many small pieces of text (commands, prompts, configs, snippets) and need a zero-friction capture-and-retrieve loop.

**Success criteria:**
- Copy text → `Ctrl+Shift+S` → type title → Enter → saved
- `Alt+Space` → search across title/content/tags/pinyin → Enter copies and closes
- System tray with quick actions
- Deep dark theme, Raycast-like minimal UI
- 100% local data, no cloud sync
- Stable v1.0.1 long-term use

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri 2.x |
| Frontend | React 19 + TypeScript |
| Styling | Tailwind CSS |
| Local DB | SQLite (via rusqlite, `bundled` + `backup` features) |
| Full-text search | SQLite FTS5 + LIKE fallback |
| Global hotkeys | `tauri-plugin-global-shortcut` |
| Clipboard | `tauri-plugin-clipboard-manager` |
| AI (optional) | Ollama HTTP API |
| Build target | Windows NSIS installer |

## Architecture

### Backend (`src-tauri/src/`)

| Module | Responsibility |
|--------|---------------|
| `lib.rs` | Tauri builder, plugin registration, AppState, window helpers |
| `commands.rs` | Tauri command functions (save/delete/update/export/import/settings/tags) |
| `clipboard.rs` | `copy_to_clipboard`, `get_current_clipboard_text`, `get_clipboard_content` |
| `hotkeys.rs` | Global shortcut registration and update |
| `tray.rs` | System tray menu |
| `settings.rs` | `Settings` struct + read helpers + seed defaults |
| `search.rs` | `search_snippets` + `list_snippets` with filter |
| `database.rs` | SQLite, FTS5, migrations, backup |
| `tags.rs` | 45 auto-tag rules |
| `ai.rs` | Ollama HTTP client with 8s timeout, JSON output |

### Frontend (`src/`)

| Path | Responsibility |
|------|----------------|
| `main.tsx` | React entry, mounts `app/App` |
| `app/App.tsx` | Window-label dispatcher (capture / search / settings) |
| `windows/CaptureWindow.tsx` | Save popup (clipboard read, type detect, dedup) |
| `windows/SearchWindow.tsx` | Search window (list, detail, keyboard nav) |
| `windows/SettingsWindow.tsx` | Settings window (system/capture/search/AI/data/tags/about) |
| `types.ts` | Shared TypeScript types |
| `lib/api.ts` | All `invoke()` calls (typed wrappers) |
| `lib/analyze.ts` | Content type detection (URL/code/prompt/text) |
| `lib/format.ts` | Re-exports time + truncation helpers |
| `lib/utils.ts` | `cn`, `highlightMatches`, `parseDuplicateError`, `classifyClipboardType` |
| `lib/mappers.ts` | `Snippet → ClipItem` |
| `hooks/useSettings.ts` | Centralized settings state |
| `hooks/useSnippets.ts` | Snippet list state with filter kind |
| `hooks/useToast.ts` | Toast state with timer |
| `hooks/useKeyboardNavigation.ts` | ↑/↓/Enter/Ctrl+C/Esc handlers |
| `components/ContentList.tsx` | Snippet list + search input |
| `components/ContentCard.tsx` | One snippet card |
| `components/DetailPanel.tsx` | Detail view with edit/copy/pin/delete |
| `components/Sidebar.tsx` | Category navigation (real filter, not query string) |
| `components/ui.tsx` | GlassPanel, Toast, Toggle, KeyboardChip, SectionHeader, TagBadge, AmbientBackground |
| `components/NumberField.tsx` | Numeric input with commit-on-blur |
| `components/typeMeta.ts` | Type → label/icon mapping |

## Database Schema

```sql
-- v0 → v1
CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    pinyin TEXT DEFAULT '',         -- v1
    type TEXT,                       -- text/code/url/prompt
    tags TEXT,                       -- comma-separated
    source_app TEXT,
    created_at DATETIME DEFAULT (datetime('now','localtime')),
    updated_at DATETIME DEFAULT (datetime('now','localtime')),
    pinned INTEGER DEFAULT 0,
    image_path TEXT                  -- v2 (kept for compat, UI disabled)
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE metadata (              -- v1+
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE VIRTUAL TABLE snippets_fts USING fts5(
    title, content, tags, pinyin,
    content=snippets,
    content_rowid=id
);

-- Triggers (snippets_ai, snippets_ad, snippets_au) keep FTS in sync
```

## Commands

```bash
# Development
npm run tauri dev          # Run in dev mode with hot reload

# Frontend type-check + production build
npm run build

# Full release build (creates NSIS installer)
npm run tauri build

# Rust unit tests
cd src-tauri && cargo test --lib
```

## Settings

| Key | Default | Range | Description |
|-----|---------|-------|-------------|
| `autostart` | `false` | bool | Open at login |
| `capture_shortcut` | `Ctrl+Shift+S` | string | Save shortcut (primary) |
| `capture_shortcut_alt` | `Alt+W` | string (empty=disable) | Save shortcut (secondary) |
| `search_shortcut` | `Alt+Space` | string | Search shortcut |
| `title_max_length` | `10` | 10/20/30 | Max title chars |
| `search_limit` | `50` | 1-500 | Max results |
| `search_debounce_ms` | `150` | 0-2000 | Search debounce |
| `auto_close_on_blur` | `true` | bool | Auto-hide on blur |
| `auto_close_delay_ms` | `150` | 0-5000 | Blur delay |
| `auto_tag_on_capture` | `true` | bool | Auto rule tags |
| `capture_text_max_length` | `50000` | 100-1000000 | Truncate threshold |
| `markdown_export_pinned_only` | `false` | bool | Filter on export |
| `ai_enabled` | `false` | bool | Enable AI tags |
| `ollama_endpoint` | `http://localhost:11434` | string | Ollama URL |
| `ollama_model` | `qwen3:4b` | string | Model name |
| `ai_tag_fallback` | `rules` | rules/none | Fallback when AI fails |
| `schema_version` | `2` | int | Internal |

## Window Labels

The Tauri backend creates three windows at startup with these labels:

| Label | Purpose | Size |
|-------|---------|------|
| `capture` | Save popup (Ctrl+Shift+S) | 560×500 |
| `search` | Main search (Alt+Space) | 900×580 |
| `settings` | Settings (from sidebar) | 560×540 |

All windows are pre-created in `setup()` with `visible(false)` to avoid on-demand creation blocking. `show_window()` is used to show/hide.

## Duplicate Detection

`save_snippet` checks `find_by_content` (LIKE on content) before insert. If a duplicate is found, the command returns an `Err(String)` in the format:

```
DUPLICATE::{"id":123,"title":"..."}
```

The frontend parses this with `parseDuplicateError()` and shows a banner with three actions: cancel, open existing, save force. The `save_snippet_force` command bypasses the duplicate check.

## AI Tagging

`auto_tag_ai` returns a structured `AiTagResult`:

```rust
struct AiTagResult {
    tags: Vec<String>,    // 0-5 tags, each ≤ 20 chars
    summary: String,      // ≤ 30 chars (not persisted in v1.0.1)
    source: String,       // "ai" or "rules"
}
```

The `summary` field is **not** persisted to the database (would require a `schema_version = 3` migration). It is returned to the frontend for display only. If Ollama is unavailable and `ai_tag_fallback = "rules"`, the 45 built-in rules are used as a fallback with `source = "rules"`.

## Boundaries

- **Always:** Use FTS5 + LIKE fallback, dark theme, modular Rust code, schema_version tracking
- **Ask first:** Adding a new Tauri plugin, changing the database schema, AI model changes
- **Never:** Store clipboard data without user confirmation, send data to external APIs (except opt-in Ollama), use Electron, commit secrets, sync to cloud

## Out of scope (v1.0.1)

- Image snippet saving (DB column kept, UI disabled)
- AI summary persistence (returned but not saved)
- Cloud sync
- Account system
- Vector database
- Complex AI knowledge base
- Multi-language UI (Chinese only for v1.0.1)
- Light theme


