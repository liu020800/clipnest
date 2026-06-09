import { useCallback, useEffect, useRef, useState } from "react";
import { Loader2, Bot } from "lucide-react";
import { api, safeOnFocusChanged, inTauri } from "../lib/api";
import { analyzeClipboardContent } from "../lib/analyze";
import { parseDuplicateError, cn } from "../lib/utils";
import { GlassPanel, KeyboardChip, SectionHeader } from "../components/ui";
import { typeMeta } from "../components/typeMeta";
import type { ClipboardAnalysis, AiTagResult } from "../types";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface DuplicateInfo {
  id: number;
  title: string;
}

export function CaptureWindow({
  onToast,
}: {
  onToast: (title: string, description?: string, tone?: "success" | "info" | "warning" | "error") => void;
}) {
  const [clipboardText, setClipboardText] = useState("");
  const [analysis, setAnalysis] = useState<ClipboardAnalysis | null>(null);
  const [title, setTitle] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [exiting, setExiting] = useState(false);
  const [aiLoading, setAiLoading] = useState(false);
  const [duplicate, setDuplicate] = useState<DuplicateInfo | null>(null);
  const [titleMaxLength, setTitleMaxLength] = useState(10);
  const inputRef = useRef<HTMLInputElement>(null);
  const aiEnabledRef = useRef(false);
  const escapePressedRef = useRef(false);
  const userEditedTitleRef = useRef(false);

  const tooLong = title.length > titleMaxLength;
  const canSave = title.trim().length > 0 && !tooLong && !saving && clipboardText.trim().length > 0;

  useEffect(() => {
    void api.getAllSettings().then((s) => {
      aiEnabledRef.current = s.ai_enabled === "true";
      const n = Number(s.title_max_length);
      if (Number.isFinite(n) && [10, 20, 30].includes(n)) {
        setTitleMaxLength(n);
      }
    }).catch(() => {});
  }, []);

  const applyText = useCallback((text: string, tags?: string[]) => {
    const a = analyzeClipboardContent(text);
    setClipboardText(text);
    setAnalysis(a);
    setTitle(a.title);
    setSelectedTags(tags ?? a.tags);
    setDuplicate(null);
    userEditedTitleRef.current = false;
  }, []);

  const readClipboard = useCallback(async (): Promise<boolean> => {
    try {
      const pendingText = await api.takePendingCaptureText();
      if (pendingText.trim()) {
        applyText(pendingText, ["OCR"]);
        return true;
      }
    } catch {
      // Pending capture text is an enhancement path; fall through to clipboard text.
    }

    try {
      const text = await api.getCurrentClipboardText();
      if (text.trim()) {
        applyText(text);
        return true;
      }
      setClipboardText("");
      setAnalysis(null);
      setTitle("");
      setSelectedTags([]);
      setDuplicate(null);
      userEditedTitleRef.current = false;
      return false;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setClipboardText("");
      setAnalysis(null);
      onToast("读取剪贴板失败", msg, "error");
      return false;
    }
  }, [applyText, onToast]);

  useEffect(() => {
    let cancelled = false;
    const tryRead = () => {
      void readClipboard().then((got) => {
        if (cancelled || got) return;
        window.setTimeout(() => {
          if (!cancelled) void readClipboard();
        }, 80);
      });
    };
    if (inTauri) {
      getCurrentWindow().setFocus().catch(() => {});
    }
    tryRead();
    window.setTimeout(() => inputRef.current?.focus(), 60);
    const unlisten = safeOnFocusChanged((focused) => {
      if (focused) {
        tryRead();
        window.setTimeout(() => inputRef.current?.focus(), 60);
      }
    });
    return () => {
      cancelled = true;
      unlisten();
    };
  }, [readClipboard]);

  const handleSaveRef = useRef<((force?: boolean) => Promise<void>) | null>(null);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        if (duplicate) {
          if (escapePressedRef.current) {
            setDuplicate(null);
            void close();
          } else {
            escapePressedRef.current = true;
            setDuplicate(null);
            setTimeout(() => {
              escapePressedRef.current = false;
            }, 500);
          }
          return;
        }
        void close();
        return;
      }
      if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        if (analysis) {
          if (userEditedTitleRef.current) {
            onToast("已使用你输入的标题", "再次按 Ctrl+Enter 确认", "info");
            userEditedTitleRef.current = false;
          } else {
            setTitle(analysis.title);
          }
          window.setTimeout(() => void handleSaveRef.current?.(), 50);
        }
        return;
      }
      if (event.key === "Enter" && !event.shiftKey) {
        if (event.target instanceof HTMLInputElement) {
          event.preventDefault();
        }
        if (duplicate) return;
        void handleSaveRef.current?.();
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      document.removeEventListener("keydown", onKeyDown, true);
    };
  }, [duplicate, analysis, onToast]);

  const handleSave = async (force = false) => {
    if (!force && !canSave) return;
    if (!clipboardText.trim()) {
      onToast("内容为空", "没有可保存的文本", "warning");
      return;
    }
    if (!force && title.trim().length === 0) {
      onToast("标题为空", "请输入标题", "warning");
      return;
    }
    if (tooLong) {
      onToast(`标题最多 ${titleMaxLength} 字`, "", "warning");
      return;
    }
    setSaving(true);
    try {
      await api.saveSnippet(title.trim(), clipboardText, selectedTags.join(",") || null);
      setSaving(false);
      onToast("已保存", title.trim(), "success");
      window.setTimeout(() => {
        setExiting(true);
        window.setTimeout(() => void api.hideCurrentWindow(), 180);
      }, 80);
    } catch (err) {
      setSaving(false);
      const msg = err instanceof Error ? err.message : String(err);
      const dup = parseDuplicateError(msg);
      if (dup) {
        setDuplicate(dup);
        return;
      }
      onToast("保存失败", msg, "error");
    }
  };

  const handleForceSave = async () => {
    if (saving) return;
    setDuplicate(null);
    setSaving(true);
    try {
      await api.saveSnippetForce(title.trim(), clipboardText, selectedTags.join(",") || null);
      setSaving(false);
      onToast("已保存副本", title.trim(), "success");
      window.setTimeout(() => {
        setExiting(true);
        window.setTimeout(() => void api.hideCurrentWindow(), 180);
      }, 80);
    } catch (err) {
      setSaving(false);
      const msg = err instanceof Error ? err.message : String(err);
      onToast("保存失败", msg, "error");
    }
  };

  useEffect(() => {
    handleSaveRef.current = handleSave;
  });

  const openExisting = () => {
    if (!duplicate) return;
    void api.hideCurrentWindow();
    onToast("已切换到搜索", "正在查找重复片段", "info");
  };

  const handleAiTag = async () => {
    if (aiLoading || !clipboardText.trim()) return;
    setAiLoading(true);
    try {
      const result: AiTagResult = await api.autoTagAi(title.trim() || "未命名", clipboardText);
      const aiTags = (result.tags || []).filter(Boolean);
      if (aiTags.length > 0) {
        setSelectedTags((cur) => {
          const merged = [...cur];
          for (const t of aiTags) {
            if (!merged.includes(t)) merged.push(t);
          }
          return merged;
        });
        onToast("AI 标签已添加", `+${aiTags.length} 个`, "success");
      } else {
        onToast("AI 未生成标签", "", "info");
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.toLowerCase().includes("ollama")) {
        onToast("AI 标签失败", "请先启动 Ollama", "error");
      } else {
        onToast("AI 标签失败", msg, "error");
      }
    } finally {
      setAiLoading(false);
    }
  };

  const close = async () => {
    await api.hideCurrentWindow();
  };

  if (!analysis) {
    return (
      <div className="capture-stage">
        <GlassPanel variant="popup" className="capture-popup">
          <div className="flex h-full flex-col items-center justify-center gap-3 text-center text-[var(--text-muted)]">
            <span className="text-[13px]">当前没有可保存的文本</span>
            <span className="text-[11px] text-[var(--text-secondary)]">
              复制文本，或使用框选识别快捷键获取屏幕文字
            </span>
            <button
              type="button"
              onClick={() => void readClipboard()}
              className="secondary-button"
              data-testid="capture-retry"
            >
              重新读取剪贴板
            </button>
          </div>
        </GlassPanel>
      </div>
    );
  }

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget) {
      void close();
    }
  };

  const meta = typeMeta(analysis.type);

  return (
    <div className="capture-stage" onClick={handleBackdropClick}>
      <GlassPanel variant="popup" className={cn("capture-popup", exiting && "capture-popup-exit")}>
        <div className="capture-top">
          <div>
            <h1 className="text-[16px] font-semibold text-[var(--text-main)]">保存文本</h1>
            <p className="mt-0.5 text-[11px] text-[var(--text-secondary)]">
              确认后保存到本地知识库
            </p>
          </div>
          <KeyboardChip>Esc 取消 · Enter 保存</KeyboardChip>
        </div>

        <div className="mt-3">
          <div className="mb-1.5 flex items-center justify-between">
            <label htmlFor="capture-title" className="text-[12px] font-medium text-[var(--text-main)]">
              名称
            </label>
            <span className={cn("text-[11px] text-[var(--text-muted)]", tooLong && "text-[var(--warning)]")}>
              {title.length}/{titleMaxLength}
            </span>
          </div>
          <input
            ref={inputRef}
            id="capture-title"
            value={title}
            maxLength={50}
            onChange={(e) => {
              userEditedTitleRef.current = true;
              setTitle(e.target.value);
            }}
            placeholder="输入标题..."
            className={cn("capture-input", tooLong && "capture-input-warning")}
          />
          {tooLong && (
            <p className="mt-1 text-[11px] text-[var(--warning)]">标题最多 {titleMaxLength} 字</p>
          )}
        </div>

        <div className="mt-3 capture-insight">
          <SectionHeader
            title="内容摘要"
            icon={<span className="h-2 w-2 rounded-full bg-[var(--accent)]" />}
          />
          <p className="line-clamp-2 text-[12.5px] text-[var(--text-secondary)]">{analysis.summary}</p>
        </div>

        <div className="mt-3">
          <SectionHeader
            title="标签"
            action={
              aiEnabledRef.current ? (
                <button
                  type="button"
                  className="copy-chip"
                  onClick={() => void handleAiTag()}
                  disabled={aiLoading}
                  title="使用 Ollama 生成标签"
                >
                  {aiLoading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Bot className="h-3.5 w-3.5" />}
                  AI 标签
                </button>
              ) : undefined
            }
          />
          <div className="flex flex-wrap gap-2">
            {selectedTags.slice(0, 5).map((tag) => (
              <span key={tag} className="tag-badge tag-badge-ai">{tag}</span>
            ))}
            {selectedTags.length > 5 && (
              <span className="tag-badge opacity-70">+{selectedTags.length - 5}</span>
            )}
            <button
              type="button"
              className="tag-custom"
              onClick={() => {
                const t = window.prompt("添加自定义标签", "");
                const trimmed = t?.trim();
                if (trimmed && !selectedTags.includes(trimmed)) {
                  setSelectedTags((cur) => [...cur, trimmed]);
                }
              }}
            >
              + 自定义
            </button>
          </div>
        </div>

        <div className="mt-3">
          <SectionHeader
            title="预览"
            action={<span className="content-type-pill">{meta.label}</span>}
          />
          <div className="preview-box">
            <pre>{clipboardText}</pre>
          </div>
        </div>

        {duplicate && (
          <div className="mt-3 duplicate-banner">
            <div className="duplicate-banner-title">这条内容已经保存过</div>
            <div className="duplicate-banner-desc">已存在: 「{duplicate.title}」</div>
            <div className="duplicate-banner-actions">
              <button type="button" className="ghost-button" onClick={() => setDuplicate(null)}>
                取消
              </button>
              <button type="button" className="secondary-button" onClick={openExisting}>
                打开已有记录
              </button>
              <button type="button" className="primary-button" onClick={() => void handleForceSave()} disabled={saving}>
                仍然保存副本
              </button>
            </div>
          </div>
        )}

        <div className="mt-auto flex items-center justify-between">
          <span className="text-[11px] text-[var(--text-muted)]">
            ↵ 保存到知识库 · Ctrl+Enter 使用自动标题
          </span>
          <div className="flex items-center gap-2">
            <button type="button" className="ghost-button" onClick={close}>
              取消
            </button>
            <button
              type="button"
              disabled={!canSave}
              onClick={() => void handleSave()}
              className="primary-button min-w-[100px]"
              data-testid="capture-save"
            >
              {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : "保存"}
            </button>
          </div>
        </div>
      </GlassPanel>
    </div>
  );
}

export default CaptureWindow;
