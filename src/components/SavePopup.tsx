import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

function generateTitle(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return "新片段";

  // URL: 取域名或路径最后一段
  if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
    try {
      const u = new URL(trimmed);
      const segs = u.pathname.split("/").filter(Boolean);
      const key = segs.length > 0 ? segs[segs.length - 1] : u.hostname;
      return key.length > 30 ? key.slice(0, 27) + "..." : key;
    } catch {
      return "链接";
    }
  }

  // 取第一个非空行
  const firstLine = trimmed.split("\n").map((l) => l.trim()).find(Boolean);
  if (!firstLine) return "新片段";
  const clean = firstLine;

  // 行内 URL：不截断
  const urlMatch = clean.match(/https?:\/\/[^\s]+/);
  if (urlMatch) {
    const url = urlMatch[0];
    try {
      const u = new URL(url);
      const host = u.hostname.replace(/^www\./, "");
      return host.length > 28 ? host.slice(0, 25) + "..." : host;
    } catch {
      return url.length > 28 ? url.slice(0, 25) + "..." : url;
    }
  }

  // 找第一个自然断句（中文标点），取断句前的内容
  const short = clean.split(/[。？！;；，,：:]/)[0].trim();
  if (short && short.length <= 28 && short.length > 0) return short;

  // 长内容：只取开头有意义的部分
  const fallback = clean.slice(0, 24);
  const space = fallback.lastIndexOf(" ");
  if (space > 4) return fallback.slice(0, space);
  return fallback + (clean.length > 24 ? "..." : "");
}

function SavePopup() {
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [tags, setTags] = useState("");
  const [saving, setSaving] = useState(false);
  const [aiLoading, setAiLoading] = useState(false);
  const [error, setError] = useState("");
  const [aiError, setAiError] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const tagsRef = useRef<HTMLInputElement>(null);

  const refresh = useCallback(async () => {
    setError("");
    setAiError("");
    try {
      const text = await invoke<string>("get_clipboard_content");
      setContent(text);
      setTitle(generateTitle(text));
    } catch {
      setError("读取剪贴板失败，请先复制一段文本。");
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setTimeout(() => inputRef.current?.focus(), 60);

    const win = getCurrentWindow();
    const unlistenPromise = win.onFocusChanged((event) => {
      if (event.payload) {
        setTags("");
        setSaving(false);
        void refresh();
        window.setTimeout(() => inputRef.current?.focus(), 60);
      }
    });

    return () => {
      window.clearTimeout(timer);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [refresh]);

  const hideWindow = async () => {
    setTags("");
    setSaving(false);
    await getCurrentWindow().hide();
  };

  const handleSave = async () => {
    if (!title.trim() || !content.trim() || saving) {
      return;
    }

    setSaving(true);
    setError("");

    try {
      await invoke("save_snippet", {
        title: title.trim(),
        content,
        tags: tags.trim() || null,
      });
      await hideWindow();
    } catch {
      setSaving(false);
      setError("保存失败，请稍后重试。");
    }
  };

  const handleAiTag = async () => {
    if (aiLoading || !content.trim()) {
      return;
    }

    setAiLoading(true);
    setAiError("");

    try {
      const result = await invoke<string>("auto_tag_ai", {
        title: title.trim(),
        content,
      });
      setTags(result);
      window.setTimeout(() => tagsRef.current?.focus(), 50);
    } catch (e) {
      setAiError(String(e));
    } finally {
      setAiLoading(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void handleSave();
    }

    if (e.key === "Escape") {
      e.preventDefault();
      void hideWindow();
    }
  };

  return (
    <div className="window-shell h-screen w-screen p-2">
      <div className="glass-panel flex h-full flex-col rounded-[20px] px-3 py-2.5">
        <div className="mb-1.5 flex items-center justify-between">
          <h1 className="text-xs font-semibold text-[var(--text-primary)]">保存</h1>
          <button
            type="button"
            onClick={() => void hideWindow()}
            className="flex h-5 w-5 items-center justify-center rounded-full text-[var(--text-dim)] hover:text-[var(--text-primary)]"
          >×</button>
        </div>

        <div className="flex-1 space-y-1.5">
          <input
            ref={inputRef}
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="标题"
            className="w-full rounded-lg border border-[var(--border-soft)] bg-[var(--bg-secondary)] px-2.5 py-1.5 text-xs text-[var(--text-primary)] outline-none placeholder-[var(--text-dim)]"
          />

          <div className="flex gap-1.5">
            <input
              ref={tagsRef}
              value={tags}
              onChange={(e) => setTags(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="标签（逗号分隔）"
              className="flex-1 rounded-lg border border-[var(--border-soft)] bg-[var(--bg-secondary)] px-2.5 py-1.5 text-xs text-[var(--text-primary)] outline-none placeholder-[var(--text-dim)]"
            />
            <button
              type="button"
              onClick={() => void handleAiTag()}
              disabled={aiLoading}
              className="rounded-lg border border-[var(--border-soft)] px-2.5 py-1.5 text-xs text-[var(--accent)] disabled:opacity-40"
            >
              {aiLoading ? "..." : "AI"}
            </button>
          </div>

          <div className="rounded-lg border border-[var(--border-soft)] bg-[var(--bg-secondary)] px-2.5 py-1.5">
            <div className="flex items-center justify-between text-[10px] text-[var(--text-dim)]">
              <span>{content.length} 字符</span>
              <span className="opacity-60">↵ 保存 ⎋ 取消</span>
            </div>
            <p className="mt-0.5 line-clamp-5 text-[11px] leading-[1.5] text-[var(--text-secondary)]">
              {content || "当前剪贴板暂无文本内容"}
            </p>
          </div>

          {(error || aiError) && (
            <div className="rounded-lg border border-[rgba(255,120,120,0.2)] bg-[rgba(120,35,35,0.22)] px-2.5 py-1 text-[11px] text-[var(--danger)]">
              {error || aiError}
            </div>
          )}
        </div>

        <div className="mt-1 text-right text-[10px] text-[var(--text-dim)] opacity-50">
          {saving ? "保存中..." : ""}
        </div>
      </div>
    </div>
  );
}

export default SavePopup;
