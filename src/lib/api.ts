// 所有 Tauri 命令的统一封装
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { Snippet, TagSummary, AiTagResult, ImportResult, OcrDoneInfo, OcrCapability, ScreenOcrRegion } from "../types";

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function safeGetCurrentWindowLabel(): string {
  if (!inTauri) return "browser";
  try {
    return getCurrentWindow().label;
  } catch {
    return "browser";
  }
}

export function safeOnFocusChanged(handler: (focused: boolean) => void): () => void {
  if (!inTauri) return () => {};
  try {
    const win = getCurrentWindow();
    const unlistenPromise = win.onFocusChanged((e) => handler(Boolean(e.payload)));
    return () => {
      void unlistenPromise.then((fn) => fn());
    };
  } catch {
    return () => {};
  }
}

const mockSnippet = (id: number, title: string, content: string, type: string, tags: string, time: string, pinned = false): Snippet => ({
  id,
  title,
  content,
  pinyin: "",
  type,
  tags,
  source_app: null,
  created_at: time,
  updated_at: time,
  pinned,
  image_path: null,
  image_dim_w: null,
  image_dim_h: null,
  ocr_status: null,
});

const MOCK_SNIPPETS: Snippet[] = [
  mockSnippet(1, "Rust 异步基础", "async fn fetch() -> Result<String> {\n  reqwest::get(\"https://api.example.com\").await?.text().await\n}", "code", "rust,async,http", "2026-06-06 10:30:00", true),
  mockSnippet(2, "TypeScript 泛型工具", "type Partial<T> = { [P in keyof T]?: T[P] };", "code", "typescript,generics", "2026-06-06 09:15:00"),
  mockSnippet(3, "GitHub Repo", "https://github.com/liu020800/clipnest", "url", "github,repo", "2026-06-05 22:00:00"),
  mockSnippet(4, "写作 Prompt", "请帮我用克制的语气改写这段文字,避免过度修饰,保留技术准确性。", "prompt", "writing,ai", "2026-06-05 18:45:00"),
  mockSnippet(5, "会议纪要", "本周重点:\n1. 迁移到 Tauri 2\n2. 完善剪贴板监听\n3. 优化搜索性能", "text", "meeting,notes", "2026-06-05 14:20:00"),
  mockSnippet(6, "NPM 脚本", "pnpm dev && tauri dev", "code", "npm,tauri", "2026-06-05 11:00:00"),
  mockSnippet(7, "PostgreSQL 连接", "psql -h localhost -U postgres -d clipnest", "code", "sql,database", "2026-06-04 16:30:00"),
  mockSnippet(8, "Tailwind 配置", "export default { content: ['./index.html', './src/**/*.{ts,tsx}'], theme: { extend: {} } }", "code", "tailwind,config", "2026-06-04 10:00:00"),
  mockSnippet(9, "设计参考", "https://linear.app — 极简编辑美学,克制的颜色,等宽字体", "url", "design,reference", "2026-06-03 20:00:00", true),
  mockSnippet(10, "用户反馈", "v1.1 测试版已开放图片捕获与本地 OCR。", "text", "feedback", "2026-06-03 15:00:00"),
];

const MOCK_SETTINGS: Record<string, string> = {
  capture_shortcut: "Ctrl+Shift+S",
  capture_shortcut_alt: "Alt+W",
  search_shortcut: "Alt+Space",
  title_max_length: "10",
  capture_text_max_length: "50000",
  search_limit: "50",
  search_debounce_ms: "150",
  auto_close_on_blur: "true",
  auto_close_delay_ms: "150",
  ollama_endpoint: "http://localhost:11434",
  ollama_model: "qwen3:4b",
  ai_tag_fallback: "rules",
  ai_enabled: "false",
  auto_tag_on_capture: "true",
  clipboard_history_enabled: "true",
  clipboard_history_max: "500",
  markdown_export_pinned_only: "false",
  screen_ocr_shortcut: "Ctrl+Shift+O",
  search_history_persist: "true",
  number_animation: "true",
  ui_density: "comfortable",
  schema_version: "3",
  autostart: "false",
};

const MOCK_TAGS: TagSummary[] = [
  { name: "rust", count: 12 },
  { name: "typescript", count: 8 },
  { name: "code", count: 24 },
  { name: "design", count: 5 },
  { name: "url", count: 7 },
  { name: "meeting", count: 3 },
  { name: "tauri", count: 6 },
  { name: "github", count: 4 },
  { name: "feedback", count: 2 },
  { name: "ai", count: 9 },
  { name: "writing", count: 3 },
  { name: "database", count: 5 },
];

function matchesMockSnippet(snippet: Snippet, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return [snippet.title, snippet.content, snippet.tags ?? ""]
    .some((value) => value.toLowerCase().includes(q));
}

export const api = {
  // === 剪贴板 ===
  copyToClipboard: (text: string) =>
    inTauri ? invoke<void>("copy_to_clipboard", { text }) : Promise.resolve(),
  pasteText: (text: string) =>
    inTauri ? invoke<void>("paste_text", { text }) : Promise.resolve(),
  getCurrentClipboardText: () =>
    inTauri
      ? invoke<string>("get_current_clipboard_text")
      : Promise.resolve("https://github.com/liu020800/clipnest — 这是一段模拟的剪贴板文本,用于在浏览器中预览 capture 窗口的布局。"),

  // === 搜索与列表 ===
  searchSnippets: (query: string) =>
    inTauri
      ? invoke<Snippet[]>("search_snippets", { query })
      : Promise.resolve(MOCK_SNIPPETS.filter((s) => matchesMockSnippet(s, query))),
  listSnippets: (args: { query: string; filterKind?: string; filterValue?: string; limit?: number }) => {
    if (!inTauri) {
      let list = [...MOCK_SNIPPETS];
      if (args.filterKind === "pinned") list = list.filter(s => s.pinned);
      else if (args.filterKind === "recent") list = list.slice(0, 5);
      else if (args.filterKind === "type" && args.filterValue) list = list.filter(s => s.type === args.filterValue);
      else if (args.filterKind === "tag" && args.filterValue) list = list.filter(s => (s.tags ?? "").toLowerCase().includes(args.filterValue!.toLowerCase()));
      else if (args.query) list = list.filter((s) => matchesMockSnippet(s, args.query));
      return Promise.resolve(list.slice(0, args.limit ?? 50));
    }
    return invoke<Snippet[]>("list_snippets", { args });
  },

  saveSnippet: (title: string, content: string, tags: string | null) =>
    inTauri
      ? invoke<number>("save_snippet", { title, content, tags })
      : Promise.resolve(Math.floor(Math.random() * 1000)),
  saveSnippetForce: (title: string, content: string, tags: string | null) =>
    inTauri
      ? invoke<number>("save_snippet_force", { title, content, tags })
      : Promise.resolve(Math.floor(Math.random() * 1000)),
  deleteSnippet: (id: number) =>
    inTauri ? invoke<void>("delete_snippet", { id }) : Promise.resolve(),
  togglePin: (id: number) =>
    inTauri ? invoke<void>("toggle_pin", { id }) : Promise.resolve(),
  updateSnippet: (
    id: number,
    title?: string,
    tags?: string | null,
    pinned?: boolean,
  ) =>
    inTauri
      ? invoke<Snippet | null>("update_snippet", { id, title, tags, pinned })
      : Promise.resolve(null),

  // === 标签 ===
  listTags: () =>
    inTauri ? invoke<TagSummary[]>("list_tags") : Promise.resolve(MOCK_TAGS),
  renameTag: (oldName: string, newName: string) =>
    inTauri ? invoke<number>("rename_tag", { oldName, newName }) : Promise.resolve(0),
  deleteTag: (name: string) =>
    inTauri ? invoke<number>("delete_tag", { name }) : Promise.resolve(0),
  mergeTags: (from: string, to: string) =>
    inTauri ? invoke<number>("merge_tags", { from, to }) : Promise.resolve(0),

  // === AI ===
  autoTagAi: (title: string, content: string) =>
    inTauri
      ? invoke<AiTagResult>("auto_tag_ai", { title, content })
      : Promise.resolve({ tags: ["auto-tag-1", "auto-tag-2"], summary: "AI 摘要 (mock)", source: "mock" }),

  // === 导入/导出 ===
  exportMarkdown: () =>
    inTauri ? invoke<string>("export_markdown") : Promise.resolve("C:/Users/.../Documents/ClipNest/export.md"),
  exportJson: () =>
    inTauri ? invoke<string>("export_json") : Promise.resolve("C:/Users/.../Documents/ClipNest/export.json"),
  importJson: (path: string) =>
    inTauri ? invoke<ImportResult>("import_json", { path }) : Promise.resolve({ imported: 0, path }),

  // === 设置 ===
  getAllSettings: () =>
    inTauri ? invoke<Record<string, string>>("get_all_settings") : Promise.resolve(MOCK_SETTINGS),
  saveSetting: (key: string, value: string) =>
    inTauri ? invoke<void>("save_setting", { key, value }) : Promise.resolve(),
  updateShortcut: (key: string, value: string) =>
    inTauri ? invoke<void>("update_shortcut", { key, value }) : Promise.resolve(),

  // === 自动启动 ===
  getAutostart: () =>
    inTauri ? invoke<boolean>("get_autostart") : Promise.resolve(false),
  setAutostart: (enable: boolean) =>
    inTauri ? invoke<void>("set_autostart", { enable }) : Promise.resolve(),

  // === 窗口 ===
  openSettings: () =>
    inTauri ? invoke<void>("open_settings") : Promise.resolve(),
  hideWindow: (label: string) =>
    inTauri ? invoke<void>("hide_window", { label }) : Promise.resolve(),

  openCapture: () =>
    inTauri ? invoke<void>("open_capture") : Promise.resolve(),

  hideCurrentWindow: async () => {
    if (inTauri) {
      const label = getCurrentWindow().label;
      try {
        await invoke<void>("hide_window", { label });
      } catch {
        try {
          await getCurrentWindow().hide();
        } catch {
          // ignore
        }
      }
    }
  },

  // === 数据库 ===
  getDbPath: () =>
    inTauri ? invoke<string>("get_db_path") : Promise.resolve("C:/Users/.../AppData/Roaming/com.copyliusq.desktop/copyliusq.db"),
  openDbDir: () =>
    inTauri ? invoke<string>("open_db_dir") : Promise.resolve("C:/Users/.../AppData/Roaming/com.copyliusq.desktop"),
  backupDatabase: () =>
    inTauri ? invoke<string>("backup_database") : Promise.resolve("C:/Users/.../backups/copyliusq-20260606-120000.db"),

  // === v1.1: screen-region OCR ===
  captureScreenOcrRegion: (region: ScreenOcrRegion) =>
    inTauri
      ? invoke<OcrDoneInfo>("capture_screen_ocr_region", { region })
      : Promise.resolve({
          text: "这是浏览器 mock 的框选 OCR 文本。",
          source: "mock",
        } as OcrDoneInfo),

  setPendingCaptureText: (text: string) =>
    inTauri ? invoke<void>("set_pending_capture_text", { text }) : Promise.resolve(),

  takePendingCaptureText: () =>
    inTauri ? invoke<string>("take_pending_capture_text") : Promise.resolve(""),

  getOcrCapability: () =>
    inTauri
      ? invoke<OcrCapability>("get_ocr_capability")
      : Promise.resolve({
          engine: "rapidocr",
          available: false,
          python_detected: false,
          python_path: null,
          script_bundled: false,
          message: "浏览器预览模式不支持 OCR",
        } as OcrCapability),

  resolveImagePath: (rel: string) =>
    inTauri
      ? invoke<string>("resolve_image_path", { rel })
      : Promise.resolve(`H:/mock/appdata/${rel}`),

  toAssetUrl: (absPath: string) =>
    inTauri ? convertFileSrc(absPath) : absPath,
};
