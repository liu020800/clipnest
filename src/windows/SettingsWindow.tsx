import { useEffect, useState, useRef, useCallback } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Settings, Loader2, FileText, Download, Upload, Pencil, GitMerge, Trash2, ExternalLink, Folder, Database } from "lucide-react";
import { api } from "../lib/api";
import { GlassPanel } from "../components/ui";
import { NumberField } from "../components/NumberField";
import { cn } from "../lib/utils";
import { useSettings } from "../hooks/useSettings";
import type { TagSummary } from "../types";

const APP_VERSION = "1.1.2";

export function SettingsWindow({
  onToast,
}: {
  onToast: (title: string, description?: string, tone?: "success" | "info" | "warning" | "error") => void;
}) {
  const settings = useSettings();
  const [exportingMd, setExportingMd] = useState(false);
  const [exportingJson, setExportingJson] = useState(false);
  const [importingJson, setImportingJson] = useState(false);
  const [tags, setTags] = useState<TagSummary[]>([]);
  const [renamingTag, setRenamingTag] = useState<string | null>(null);
  const [renameVal, setRenameVal] = useState("");
  const [dbPath, setDbPath] = useState("");
  const [stats, setStats] = useState<{ total: number; pinned: number; tags: number } | null>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);

  const refreshTags = useCallback(async () => {
    try {
      const list = await api.listTags();
      setTags(list);
    } catch {
      setTags([]);
    }
  }, []);

  const refreshStats = useCallback(async () => {
    try {
      const [snippets, tagList] = await Promise.all([
        api.searchSnippets(""),
        api.listTags(),
      ]);
      setStats({
        total: snippets.length,
        pinned: snippets.filter((s) => s.pinned).length,
        tags: tagList.length,
      });
    } catch (e) {
      console.error(e);
    }
  }, []);

  const refreshDbPath = useCallback(async () => {
    try {
      const path = await api.getDbPath();
      setDbPath(path);
    } catch (e) {
      console.error(e);
    }
  }, []);

  useEffect(() => {
    void refreshTags();
    void refreshStats();
    void refreshDbPath();
  }, [refreshTags, refreshStats, refreshDbPath]);

  useEffect(() => {
    if (renamingTag && renameInputRef.current) {
      renameInputRef.current.focus();
      renameInputRef.current.select();
    }
  }, [renamingTag]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") void api.hideCurrentWindow();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const closeWindow = () => {
    void api.hideCurrentWindow();
  };

  const toggleAutostart = async (next: boolean) => {
    try {
      await api.setAutostart(next);
      await settings.saveOne("autostart", next ? "true" : "false");
      onToast(next ? "已启用开机自启" : "已禁用开机自启", undefined, "success");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      onToast("设置失败", msg, "error");
    }
  };

  const updateShortcut = async (key: "capture_shortcut" | "capture_shortcut_alt" | "search_shortcut" | "screen_ocr_shortcut", value: string) => {
    const trimmed = value.trim();
    if (!trimmed && key === "capture_shortcut") {
      onToast("快捷键不能为空", "", "warning");
      return;
    }
    try {
      await api.updateShortcut(key, trimmed);
      await settings.saveOne(key, trimmed);
      onToast("已更新", "快捷键已即时生效", "success");
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      onToast("更新失败", msg, "error");
    }
  };

  const handleExportMd = async () => {
    if (exportingMd) return;
    setExportingMd(true);
    try {
      const path = await api.exportMarkdown();
      onToast("导出成功", path, "success");
    } catch (e) {
      onToast("导出失败", e instanceof Error ? e.message : String(e), "error");
    } finally {
      setExportingMd(false);
    }
  };

  const handleExportJson = async () => {
    if (exportingJson) return;
    setExportingJson(true);
    try {
      const path = await api.exportJson();
      onToast("JSON 导出成功", path, "success");
    } catch (e) {
      onToast("导出失败", e instanceof Error ? e.message : String(e), "error");
    } finally {
      setExportingJson(false);
    }
  };

  const handleImportJson = async () => {
    if (importingJson) return;
    let selected: string | string[] | null = null;
    try {
      selected = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: "ClipNest JSON", extensions: ["json"] }],
      });
    } catch (e) {
      onToast("打开文件选择器失败", e instanceof Error ? e.message : String(e), "error");
      return;
    }
    if (!selected || Array.isArray(selected)) return;
    setImportingJson(true);
    try {
      const result = await api.importJson(selected);
      onToast(`已导入 ${result.imported} 条`, result.path, "success");
      await refreshTags();
      await refreshStats();
    } catch (e) {
      onToast("导入失败", e instanceof Error ? e.message : String(e), "error");
    } finally {
      setImportingJson(false);
    }
  };

  const handleOpenDbDir = async () => {
    try {
      const dir = await api.openDbDir();
      onToast("已打开数据库目录", dir, "success");
    } catch (e) {
      onToast("打开目录失败", e instanceof Error ? e.message : String(e), "error");
    }
  };

  const handleBackupDb = async () => {
    try {
      const dest = await api.backupDatabase();
      onToast("已备份数据库", dest, "success");
    } catch (e) {
      onToast("备份失败", e instanceof Error ? e.message : String(e), "error");
    }
  };

  const startRename = (name: string) => {
    setRenamingTag(name);
    setRenameVal(name);
  };

  const cancelRename = () => {
    setRenamingTag(null);
    setRenameVal("");
  };

  const commitRename = async () => {
    if (!renamingTag || !renameVal.trim() || renameVal.trim() === renamingTag) {
      cancelRename();
      return;
    }
    try {
      const affected = await api.renameTag(renamingTag, renameVal.trim());
      onToast("已重命名", `影响 ${affected} 条记录`, "success");
      cancelRename();
      await refreshTags();
      await refreshStats();
    } catch (e) {
      onToast("重命名失败", e instanceof Error ? e.message : String(e), "error");
    }
  };

  const handleDeleteTag = async (name: string) => {
    if (!window.confirm(`确定删除标签「${name}」?所有引用此标签的记录会移除此标签。`)) return;
    try {
      const affected = await api.deleteTag(name);
      onToast("已删除", `影响 ${affected} 条记录`, "success");
      await refreshTags();
      await refreshStats();
    } catch (e) {
      onToast("删除失败", e instanceof Error ? e.message : String(e), "error");
    }
  };

  const handleMergeTag = async (from: string) => {
    const to = window.prompt(`将「${from}」合并到哪个标签?`);
    if (!to || to.trim() === from) return;
    try {
      const affected = await api.mergeTags(from, to.trim());
      onToast("已合并", `影响 ${affected} 条记录`, "success");
      await refreshTags();
      await refreshStats();
    } catch (e) {
      onToast("合并失败", e instanceof Error ? e.message : String(e), "error");
    }
  };

  if (!settings.loaded) {
    return (
      <div className="settings-stage">
        <GlassPanel variant="window" className="settings-window">
          <div className="flex h-full flex-col items-center justify-center gap-2 text-[var(--text-muted)]">
            <Loader2 className="h-5 w-5 animate-spin" />
            加载设置中...
          </div>
        </GlassPanel>
      </div>
    );
  }

  const isAutostartOn = settings.getBool("autostart", false);
  const isAiOn = settings.getBool("ai_enabled", false);
  const isAutoTagOn = settings.getBool("auto_tag_on_capture", true);
  const isAutoCloseOn = settings.getBool("auto_close_on_blur", true);
  const isClipboardHistoryOn = settings.getBool("clipboard_history_enabled", true);
  const captureShortcut = settings.get("capture_shortcut", "Ctrl+Shift+S");
  const captureShortcutAlt = settings.get("capture_shortcut_alt", "Alt+W");
  const searchShortcut = settings.get("search_shortcut", "Alt+Space");
  const screenOcrShortcut = settings.get("screen_ocr_shortcut", "Ctrl+Shift+O");
  const titleMax = settings.getNum("title_max_length", 10);
  const captureMax = settings.getNum("capture_text_max_length", 50000);
  const clipboardHistoryMax = settings.getNum("clipboard_history_max", 500);
  const searchLimit = settings.getNum("search_limit", 50);
  const debounceMs = settings.getNum("search_debounce_ms", 150);
  const autoCloseDelay = settings.getNum("auto_close_delay_ms", 150);
  const ollamaEndpoint = settings.get("ollama_endpoint", "http://localhost:11434");
  const ollamaModel = settings.get("ollama_model", "qwen3:4b");
  const aiTagFallback = settings.get("ai_tag_fallback", "rules");
  const isMarkdownPinnedOnly = settings.getBool("markdown_export_pinned_only", false);

  return (
    <div className="settings-stage">
      <GlassPanel variant="window" className="settings-window">
        <div className="settings-titlebar">
          <div className="flex items-center gap-2">
            <Settings className="h-4 w-4 text-[var(--accent)]" />
            <span className="text-[13px] font-semibold text-[var(--text-primary)]">ClipNest 设置</span>
          </div>
          <button type="button" className="window-close" onClick={closeWindow} aria-label="关闭">
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="settings-content">
          {/* 系统 */}
          <section className="settings-section">
            <div className="settings-section-title">系统</div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">开机自启</div>
                <div className="settings-row-desc">登录 Windows 时自动启动 ClipNest</div>
              </div>
              <button
                type="button"
                className={cn("toggle", isAutostartOn && "toggle-on")}
                onClick={() => void toggleAutostart(!isAutostartOn)}
                aria-pressed={isAutostartOn}
              >
                <span className="toggle-knob" />
              </button>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">保存快捷键(主)</div>
                <div className="settings-row-desc">默认 Ctrl+Shift+S,失败会立即提示</div>
              </div>
              <div className="settings-inline-input">
                <input
                  type="text"
                  defaultValue={captureShortcut}
                  key={`cs-${captureShortcut}`}
                  className="settings-text-input"
                  placeholder="Ctrl+Shift+S"
                  onBlur={(e) => {
                    const v = e.target.value.trim();
                    if (v && v !== captureShortcut) void updateShortcut("capture_shortcut", v);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  }}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">保存快捷键(备用)</div>
                <div className="settings-row-desc">默认 Alt+W,留空表示禁用</div>
              </div>
              <div className="settings-inline-input">
                <input
                  type="text"
                  defaultValue={captureShortcutAlt}
                  key={`csa-${captureShortcutAlt}`}
                  className="settings-text-input"
                  placeholder="Alt+W (留空禁用)"
                  onBlur={(e) => {
                    const v = e.target.value.trim();
                    if (v !== captureShortcutAlt) void updateShortcut("capture_shortcut_alt", v);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  }}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">搜索快捷键</div>
                <div className="settings-row-desc">默认 Alt+Space,留空可禁用</div>
              </div>
              <div className="settings-inline-input">
                <input
                  type="text"
                  defaultValue={searchShortcut}
                  key={`ss-${searchShortcut}`}
                  className="settings-text-input"
                  placeholder="Alt+Space (留空禁用)"
                  onBlur={(e) => {
                    const v = e.target.value.trim();
                    // 允许清空(禁用),只要值有变化就提交。
                    if (v !== searchShortcut) void updateShortcut("search_shortcut", v);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  }}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">框选识别快捷键</div>
                <div className="settings-row-desc">默认 Ctrl+Shift+O,拖拽框选屏幕文字后进入保存</div>
              </div>
              <div className="settings-inline-input">
                <input
                  type="text"
                  defaultValue={screenOcrShortcut}
                  key={`ocr-${screenOcrShortcut}`}
                  className="settings-text-input"
                  placeholder="Ctrl+Shift+O (留空禁用)"
                  onBlur={(e) => {
                    const v = e.target.value.trim();
                    if (v !== screenOcrShortcut) void updateShortcut("screen_ocr_shortcut", v);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  }}
                />
              </div>
            </div>
          </section>

          {/* 捕获 */}
          <section className="settings-section">
            <div className="settings-section-title">捕获</div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">保存时自动打规则标签</div>
                <div className="settings-row-desc">无标签时根据 45 条内置规则生成</div>
              </div>
              <button
                type="button"
                className={cn("toggle", isAutoTagOn && "toggle-on")}
                onClick={() => settings.saveOne("auto_tag_on_capture", isAutoTagOn ? "false" : "true")}
              >
                <span className="toggle-knob" />
              </button>
            </div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">自动记录剪贴板</div>
                <div className="settings-row-desc">复制文本后自动加入历史,可从搜索窗口直接粘贴</div>
              </div>
              <button
                type="button"
                className={cn("toggle", isClipboardHistoryOn && "toggle-on")}
                onClick={() => settings.saveOne("clipboard_history_enabled", isClipboardHistoryOn ? "false" : "true")}
                aria-pressed={isClipboardHistoryOn}
              >
                <span className="toggle-knob" />
              </button>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">剪贴板历史保留数</div>
                <div className="settings-row-desc">仅清理自动记录且未置顶的历史,范围 50–5000</div>
              </div>
              <div className="settings-inline-input">
                <NumberField
                  value={clipboardHistoryMax}
                  min={50}
                  max={5000}
                  onCommit={(n) => void settings.saveOne("clipboard_history_max", String(n))}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">标题最大字数</div>
                <div className="settings-row-desc">默认 10,可选 10/20/30</div>
              </div>
              <div className="settings-inline-input">
                <select
                  className="settings-text-input"
                  value={String(titleMax)}
                  onChange={(e) => void settings.saveOne("title_max_length", e.target.value)}
                >
                  <option value="10">10</option>
                  <option value="20">20</option>
                  <option value="30">30</option>
                </select>
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">单条最大字符数</div>
                <div className="settings-row-desc">超过则截断(默认 50000),范围 100–1,000,000</div>
              </div>
              <div className="settings-inline-input">
                <NumberField
                  value={captureMax}
                  min={100}
                  max={1_000_000}
                  onCommit={(n) => void settings.saveOne("capture_text_max_length", String(n))}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">OCR 引擎</div>
                <div className="settings-row-desc">RapidOCR 本地识别,失败时回退 WeChatOCR</div>
              </div>
              <div className="settings-inline-input">
                <span className="settings-text-input" style={{ color: "var(--accent)" }}>
                  RapidOCR
                </span>
              </div>
            </div>
          </section>

          {/* 搜索 */}
          <section className="settings-section">
            <div className="settings-section-title">搜索</div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">最大结果数</div>
                <div className="settings-row-desc">单次搜索返回上限(默认 50),范围 1–500</div>
              </div>
              <div className="settings-inline-input">
                <NumberField
                  value={searchLimit}
                  min={1}
                  max={500}
                  onCommit={(n) => void settings.saveOne("search_limit", String(n))}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">输入防抖(毫秒)</div>
                <div className="settings-row-desc">默认 150,范围 0–2000</div>
              </div>
              <div className="settings-inline-input">
                <NumberField
                  value={debounceMs}
                  min={0}
                  max={2000}
                  onCommit={(n) => void settings.saveOne("search_debounce_ms", String(n))}
                />
              </div>
            </div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">失焦自动关闭</div>
                <div className="settings-row-desc">搜索窗口失焦后自动隐藏</div>
              </div>
              <button
                type="button"
                className={cn("toggle", isAutoCloseOn && "toggle-on")}
                onClick={() => settings.saveOne("auto_close_on_blur", isAutoCloseOn ? "false" : "true")}
              >
                <span className="toggle-knob" />
              </button>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">失焦延迟(毫秒)</div>
                <div className="settings-row-desc">默认 150,范围 0–5000</div>
              </div>
              <div className="settings-inline-input">
                <NumberField
                  value={autoCloseDelay}
                  min={0}
                  max={5000}
                  onCommit={(n) => void settings.saveOne("auto_close_delay_ms", String(n))}
                />
              </div>
            </div>
          </section>

          {/* AI */}
          <section className="settings-section">
            <div className="settings-section-title">AI 标签</div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">启用 AI 标签</div>
                <div className="settings-row-desc">默认关闭,需要本地 Ollama</div>
              </div>
              <button
                type="button"
                className={cn("toggle", isAiOn && "toggle-on")}
                onClick={() => settings.saveOne("ai_enabled", isAiOn ? "false" : "true")}
              >
                <span className="toggle-knob" />
              </button>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">Ollama 服务地址</div>
                <div className="settings-row-desc">默认 http://localhost:11434</div>
              </div>
              <div className="settings-inline-input">
                <input
                  type="text"
                  defaultValue={ollamaEndpoint}
                  key={`ep-${ollamaEndpoint}`}
                  className="settings-text-input"
                  placeholder="http://localhost:11434"
                  onBlur={(e) => {
                    const v = e.target.value.trim();
                    if (v && v !== ollamaEndpoint) void settings.saveOne("ollama_endpoint", v);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  }}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">Ollama 模型</div>
                <div className="settings-row-desc">默认 qwen3:4b</div>
              </div>
              <div className="settings-inline-input">
                <input
                  type="text"
                  defaultValue={ollamaModel}
                  key={`mdl-${ollamaModel}`}
                  className="settings-text-input"
                  placeholder="qwen3:4b"
                  onBlur={(e) => {
                    const v = e.target.value.trim();
                    if (v && v !== ollamaModel) void settings.saveOne("ollama_model", v);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                  }}
                />
              </div>
            </div>

            <div className="settings-row settings-row-column">
              <div className="settings-row-info">
                <div className="settings-row-label">失败时回退</div>
                <div className="settings-row-desc">Ollama 不可用时是否回退到规则标签</div>
              </div>
              <div className="settings-inline-input">
                <select
                  className="settings-text-input"
                  value={aiTagFallback}
                  onChange={(e) => void settings.saveOne("ai_tag_fallback", e.target.value)}
                >
                  <option value="rules">回退到规则标签</option>
                  <option value="none">不进行回退</option>
                </select>
              </div>
            </div>
          </section>

          {/* 数据 */}
          <section className="settings-section">
            <div className="settings-section-title">数据</div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">知识库统计</div>
                <div className="settings-row-desc">
                  {stats ? `共 ${stats.total} 条 · 已固定 ${stats.pinned} 条 · 标签 ${stats.tags} 个` : "正在加载..."}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">数据库位置</div>
                <div className="settings-row-desc settings-row-mono" title={dbPath}>{dbPath || "—"}</div>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  className="settings-action"
                  onClick={() => void handleOpenDbDir()}
                  title="在文件管理器中打开"
                >
                  <Folder className="h-3.5 w-3.5" />
                  打开目录
                </button>
                <button
                  type="button"
                  className="settings-action"
                  onClick={() => void handleBackupDb()}
                  title="立即创建一份带时间戳的备份"
                >
                  <Database className="h-3.5 w-3.5" />
                  立即备份
                </button>
              </div>
            </div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">Markdown 导出范围</div>
                <div className="settings-row-desc">仅导出已固定 vs 全部片段</div>
              </div>
              <button
                type="button"
                className={cn("toggle", isMarkdownPinnedOnly && "toggle-on")}
                onClick={() => settings.saveOne("markdown_export_pinned_only", isMarkdownPinnedOnly ? "false" : "true")}
              >
                <span className="toggle-knob" />
              </button>
            </div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">导出为 Markdown</div>
                <div className="settings-row-desc">保存到 文档\ClipNest 目录</div>
              </div>
              <button
                type="button"
                className="settings-action"
                onClick={() => void handleExportMd()}
                disabled={exportingMd}
              >
                {exportingMd ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <FileText className="h-3.5 w-3.5" />}
                立即导出
              </button>
            </div>

            <div className="settings-row">
              <div className="settings-row-info">
                <div className="settings-row-label">JSON 导出 / 导入</div>
                <div className="settings-row-desc">用于备份或跨设备迁移</div>
              </div>
              <div className="flex gap-2">
                <button
                  type="button"
                  className="settings-action"
                  onClick={() => void handleExportJson()}
                  disabled={exportingJson}
                >
                  {exportingJson ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Download className="h-3.5 w-3.5" />}
                  导出 JSON
                </button>
                <button
                  type="button"
                  className="settings-action"
                  onClick={() => void handleImportJson()}
                  disabled={importingJson}
                >
                  {importingJson ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Upload className="h-3.5 w-3.5" />}
                  导入 JSON
                </button>
              </div>
            </div>
          </section>

          {/* 标签管理 */}
          <section className="settings-section">
            <div className="settings-section-title">标签管理</div>
            {tags.length === 0 ? (
              <div className="settings-row-desc">暂无标签</div>
            ) : (
              <ul className="settings-tag-list">
                {tags.map((t) => (
                  <li key={t.name} className="settings-tag-item">
                    {renamingTag === t.name ? (
                      <input
                        ref={renameInputRef}
                        type="text"
                        value={renameVal}
                        onChange={(e) => setRenameVal(e.target.value)}
                        onBlur={() => void commitRename()}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                          if (e.key === "Escape") cancelRename();
                        }}
                        className="settings-text-input flex-1"
                      />
                    ) : (
                      <>
                        <span className="settings-tag-name">{t.name}</span>
                        <span className="settings-tag-count">{t.count}</span>
                        <div className="flex gap-1">
                          <button
                            type="button"
                            className="settings-tag-btn"
                            onClick={() => startRename(t.name)}
                            title="重命名"
                          >
                            <Pencil className="h-3 w-3" />
                          </button>
                          <button
                            type="button"
                            className="settings-tag-btn"
                            onClick={() => void handleMergeTag(t.name)}
                            title="合并到其他标签"
                          >
                            <GitMerge className="h-3 w-3" />
                          </button>
                          <button
                            type="button"
                            className="settings-tag-btn settings-tag-btn-danger"
                            onClick={() => void handleDeleteTag(t.name)}
                            title="删除"
                          >
                            <Trash2 className="h-3 w-3" />
                          </button>
                        </div>
                      </>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </section>

          {/* 关于 */}
          <section className="settings-section">
            <div className="settings-section-title">关于</div>
            <div className="settings-about">
              <div className="settings-about-row">
                <span>版本</span>
                <span>{APP_VERSION}</span>
              </div>
              <div className="settings-about-row">
                <span>数据目录</span>
                <code className="text-[11px]">{dbPath || "—"}</code>
              </div>
              <div className="settings-about-row">
                <span>仓库</span>
                <a
                  href="https://github.com/liu020800/clipnest"
                  target="_blank"
                  rel="noreferrer"
                  className="text-[var(--accent)] inline-flex items-center gap-1"
                >
                  github.com/liu020800/clipnest
                  <ExternalLink className="h-3 w-3" />
                </a>
              </div>
              <div className="settings-about-row">
                <span>许可证</span>
                <span className="text-[var(--text-muted)]">仅供个人使用</span>
              </div>
            </div>
          </section>
        </div>
      </GlassPanel>
    </div>
  );
}
