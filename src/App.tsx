import { useCallback, useEffect, useRef, useState, type HTMLAttributes, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Archive,
  Bot,
  Check,
  Clipboard,
  Code2,
  Copy,
  Database,
  FileText,
  Globe2,
  Image,
  Link2,
  Loader2,
  MoreHorizontal,
  PenLine,
  Plus,
  Search,
  Settings,
  Sparkles,
  Trash2,
} from "lucide-react";
import {
  analyzeClipboardContent,
  type ClipboardAnalysis,
  type ClipboardContentType,
} from "./mockAI";

type GlassVariant = "popup" | "window" | "card" | "ai";
type ToastTone = "success" | "info";

interface Snippet {
  id: number;
  title: string;
  content: string;
  pinyin: string;
  type: string | null;
  tags: string | null;
  source_app: string | null;
  created_at: string;
  updated_at: string;
  pinned: boolean;
}

interface ClipItem {
  id: number;
  title: string;
  summary: string;
  content: string;
  tags: string[];
  time: string;
  savedAt: string;
  type: ClipboardContentType;
  analysis: ClipboardAnalysis;
  pinned: boolean;
}

function snippetToClipItem(s: Snippet): ClipItem {
  const analysis = analyzeClipboardContent(s.content);
  const type: ClipboardContentType =
    s.type === "url" ? "url" : s.type === "code" ? "code" : "text";
  const tags = s.tags
    ? s.tags.split(",").map((t) => t.trim()).filter(Boolean)
    : analysis.tags;
  return {
    id: s.id,
    title: s.title,
    summary: analysis.summary,
    content: s.content,
    tags,
    time: formatTime(s.created_at),
    savedAt: s.created_at,
    type,
    analysis: { ...analysis, title: s.title, tags },
    pinned: s.pinned,
  };
}

function formatTime(raw: string): string {
  const d = new Date(raw.replace(" ", "T"));
  if (Number.isNaN(d.getTime())) return raw;
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffMin = Math.floor(diffMs / 60000);
  if (diffMin < 1) return "刚刚";
  if (diffMin < 60) return `${diffMin} 分钟前`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr} 小时前`;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(d);
}

function typeMeta(type: ClipboardContentType) {
  if (type === "code") return { label: "代码片段", icon: Code2 };
  if (type === "url") return { label: "网址", icon: Link2 };
  if (type === "image") return { label: "图片", icon: Image };
  return { label: "普通文本", icon: FileText };
}

function cn(...classes: Array<string | false | undefined>) {
  return classes.filter(Boolean).join(" ");
}

export function AmbientBackground() {
  return <div className="ambient-background" aria-hidden="true" />;
}

export function GlassPanel({
  children,
  className = "",
  variant = "card",
  ...props
}: {
  children: ReactNode;
  className?: string;
  variant?: GlassVariant;
} & HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("glass-panel", `glass-panel-${variant}`, className)} {...props}>{children}</div>;
}

export function KeyboardChip({ children }: { children: ReactNode }) {
  return <span className="keyboard-chip">{children}</span>;
}

export function SectionHeader({
  title,
  subtitle,
  action,
  icon,
}: {
  title: string;
  subtitle?: ReactNode;
  action?: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <div className="section-header">
      <div>
        <div className="section-header-title">
          {icon}
          {title}
        </div>
        {subtitle && <div className="section-header-subtitle">{subtitle}</div>}
      </div>
      {action}
    </div>
  );
}

export function TagBadge({
  children,
  selected = false,
  ai = false,
  onClick,
}: {
  children: ReactNode;
  selected?: boolean;
  ai?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn("tag-badge", ai && "tag-badge-ai", selected && "tag-badge-selected")}
    >
      {children}
    </button>
  );
}

export function Toast({
  show,
  title,
  description,
  tone = "success",
  onClick,
}: {
  show: boolean;
  title: string;
  description?: string;
  tone?: ToastTone;
  onClick?: () => void;
}) {
  return (
    <button type="button" onClick={onClick} className={cn("toast", `toast-${tone}`, show && "toast-visible")}>
      <div className="toast-icon">
        <Check className="h-4 w-4" />
      </div>
      <div className="text-left">
        <div className="toast-title">{title}</div>
        {description && <div className="toast-description">{description}</div>}
      </div>
    </button>
  );
}

export function CapturePopup({ onToast }: { onToast: (title: string, description?: string) => void }) {
  const [clipboardContent, setClipboardContent] = useState("");
  const [analysis, setAnalysis] = useState<ClipboardAnalysis | null>(null);
  const [title, setTitle] = useState("");
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [exiting, setExiting] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const tooLong = title.length > 30;
  const canSave = title.trim().length > 0 && !tooLong && !saving;

  const refreshClipboard = useCallback(async () => {
    try {
      const text = await invoke<string>("get_clipboard_content");
      setClipboardContent(text);
      const a = analyzeClipboardContent(text);
      setAnalysis(a);
      setTitle(a.title);
      setSelectedTags(a.tags);
    } catch {
      setClipboardContent("");
    }
  }, []);

  useEffect(() => {
    void refreshClipboard();
    window.setTimeout(() => inputRef.current?.focus(), 60);
    const win = getCurrentWindow();
    const unlisten = win.onFocusChanged((e) => {
      if (e.payload) {
        void refreshClipboard();
        window.setTimeout(() => inputRef.current?.focus(), 60);
      }
    });
    return () => { void unlisten.then((fn) => fn()); };
  }, [refreshClipboard]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        void getCurrentWindow().hide();
      }
      if (event.key === "Enter" && !event.shiftKey) {
        event.preventDefault();
        void handleSave();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  const handleSave = async () => {
    if (!canSave || !clipboardContent.trim()) return;
    setSaving(true);
    try {
      await invoke("save_snippet", {
        title: title.trim(),
        content: clipboardContent,
        tags: selectedTags.join(",") || null,
      });
      setExiting(true);
      setSaving(false);
      onToast("已保存", `${title.trim()}`);
      window.setTimeout(() => {
        setExiting(false);
        void getCurrentWindow().hide();
      }, 380);
    } catch {
      setSaving(false);
      onToast("保存失败", "请稍后重试");
    }
  };

  const toggleTag = (tag: string) => {
    setSelectedTags((cur) => (cur.includes(tag) ? cur.filter((t) => t !== tag) : [...cur, tag]));
  };

  if (!analysis) {
    return (
      <div className="capture-stage">
        <GlassPanel variant="popup" className="capture-popup">
          <div className="flex h-full items-center justify-center text-[var(--text-muted)]">读取剪贴板中...</div>
        </GlassPanel>
      </div>
    );
  }

  const meta = typeMeta(analysis.type);

  return (
    <div className="capture-stage">
      <GlassPanel variant="popup" className={cn("capture-popup", exiting && "capture-popup-exit")}>
        <div className="capture-top">
          <div>
            <h1 className="text-[16px] font-semibold text-[var(--text-main)]">保存剪贴板</h1>
            <p className="mt-0.5 text-[11px] text-[var(--text-secondary)]">确认后保存到本地知识库</p>
          </div>
          <KeyboardChip>Esc 取消</KeyboardChip>
        </div>

        <div className="mt-3">
          <div className="mb-1.5 flex items-center justify-between">
            <label htmlFor="capture-title" className="text-[12px] font-medium text-[var(--text-main)]">
              名称
            </label>
            <span className={cn("text-[11px] text-[var(--text-muted)]", tooLong && "text-[var(--warning)]")}>
              {title.length}/30
            </span>
          </div>
          <input
            ref={inputRef}
            id="capture-title"
            value={title}
            maxLength={50}
            onChange={(event) => setTitle(event.target.value)}
            placeholder="输入标题..."
            className={cn("capture-input", tooLong && "capture-input-warning")}
          />
        </div>

        <div className="mt-3 capture-insight">
          <SectionHeader
            title="内容摘要"
            icon={<Sparkles className="h-3.5 w-3.5 text-[var(--accent)]" />}
          />
          <p className="line-clamp-2">{analysis.summary}</p>
        </div>

        <div className="mt-3">
          <SectionHeader title="标签" />
          <div className="flex flex-wrap gap-2">
            {selectedTags.slice(0, 3).map((tag) => (
              <TagBadge key={tag} ai selected onClick={() => toggleTag(tag)}>
                {tag}
              </TagBadge>
            ))}
            {selectedTags.length > 3 && (
              <span className="tag-badge opacity-70">+{selectedTags.length - 3}</span>
            )}
            <button type="button" className="tag-custom">
              <Plus className="h-3.5 w-3.5" />
              自定义
            </button>
          </div>
        </div>

        <div className="mt-3">
          <SectionHeader
            title="预览"
            action={
              <div className="flex items-center gap-2">
                <span className="content-type-pill">{meta.label}</span>
                <button type="button" className="copy-chip" onClick={() => void navigator.clipboard.writeText(clipboardContent)}>
                  <Copy className="h-3.5 w-3.5" />
                  复制
                </button>
              </div>
            }
          />
          <div className="preview-box">
            <pre>{clipboardContent}</pre>
          </div>
        </div>

        <div className="mt-auto flex items-center justify-between">
          <span className="text-[11px] text-[var(--text-muted)]">↵ 保存到知识库</span>
          <div className="flex items-center gap-2">
            <button type="button" className="ghost-button" onClick={() => void getCurrentWindow().hide()}>
              取消
            </button>
            <button type="button" disabled={!canSave} onClick={() => void handleSave()} className="primary-button min-w-[124px]">
              {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : "保存"}
            </button>
          </div>
        </div>
      </GlassPanel>
    </div>
  );
}

export function Sidebar({ active, onNavigate, onSettings, onExport }: { active: string; onNavigate: (v: string) => void; onSettings?: () => void; onExport?: () => void }) {
  const groups = [
    [["全部", Archive], ["最近保存", Clipboard]],
    [["AI", Bot], ["NAS", Database], ["代码", Code2], ["教程", FileText]],
    [["网址", Globe2], ["图片", Image]],
  ] as const;

  const renderItem = ([label, Icon]: readonly [string, typeof Archive]) => (
    <button key={label} type="button" onClick={() => onNavigate(label)} className={cn("sidebar-item", active === label && "sidebar-item-active")}>
      <Icon className="h-[15px] w-[15px]" />
      {label}
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="mb-5">
        <div className="flex items-center gap-2.5">
          <div className="app-mark">
            <Clipboard className="h-4 w-4" />
          </div>
          <div>
            <div className="text-[17px] font-semibold text-[var(--text-main)]">ClipNest</div>
            <div className="text-[11px] text-[var(--text-muted)]">AI Clipboard Memory</div>
          </div>
        </div>
      </div>

      <nav className="mt-5">
        {groups.map((group, index) => (
          <div key={index} className="sidebar-group">
            {group.map(renderItem)}
          </div>
        ))}
      </nav>

      <div className="mt-auto space-y-1 pt-5">
        <button type="button" className="sidebar-item" onClick={onExport}>
          <FileText className="h-[15px] w-[15px]" />
          导出 Markdown
        </button>
        <button type="button" className="sidebar-item" onClick={onSettings}>
          <Settings className="h-[15px] w-[15px]" />
          设置
        </button>
      </div>
    </aside>
  );
}

export function ContentCard({ item, selected, onClick }: { item: ClipItem; selected: boolean; onClick: () => void }) {
  const meta = typeMeta(item.type);
  const Icon = meta.icon;
  const visibleTags = item.tags.slice(0, 3);
  const hiddenCount = Math.max(item.tags.length - visibleTags.length, 0);
  return (
    <button type="button" onClick={onClick} className={cn("content-card", selected && "content-card-selected")}>
      <div className="flex items-start gap-3">
        <div className="type-icon">
          <Icon className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1 text-left">
          <div className="flex items-center justify-between gap-3">
            <h3 className="truncate text-[15px] font-semibold text-[var(--text-main)]">{item.title}</h3>
            <span className="shrink-0 text-[11px] text-[var(--text-muted)]">{item.time}</span>
          </div>
          <p className="mt-2 line-clamp-2 text-[13px] leading-5 text-[var(--text-secondary)]">{item.summary}</p>
          <div className="mt-3 flex flex-wrap gap-1.5">
            {visibleTags.map((tag) => (
              <span key={tag} className="mini-tag">{tag}</span>
            ))}
            {hiddenCount > 0 && <span className="mini-tag mini-tag-more">+{hiddenCount}</span>}
          </div>
        </div>
      </div>
    </button>
  );
}

export function ContentList({
  items,
  selectedId,
  onSelect,
  query,
  onQueryChange,
}: {
  items: ClipItem[];
  selectedId: number;
  onSelect: (id: number) => void;
  query: string;
  onQueryChange: (q: string) => void;
}) {
  return (
    <section className="content-list">
      <div className="content-list-top">
        <div className="search-input-shell">
          <Search className="h-4 w-4 text-[var(--text-muted)]" />
          <input
            value={query}
            onChange={(event) => onQueryChange(event.target.value)}
            placeholder="搜索知识库..."
            className="min-w-0 flex-1 bg-transparent text-[14px] text-[var(--text-main)] outline-none placeholder:text-[var(--text-muted)]"
          />
        </div>
      </div>

      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-4">
        {items.length === 0 && (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
            <p className="text-sm text-[var(--text-secondary)]">{query ? "无匹配结果" : "暂无保存内容"}</p>
            <p className="text-[12px] text-[var(--text-muted)]">{query ? "试试更短的关键词" : "Alt+W 保存剪贴板内容"}</p>
          </div>
        )}
        {items.map((item) => (
          <ContentCard key={item.id} item={item} selected={item.id === selectedId} onClick={() => onSelect(item.id)} />
        ))}
      </div>
    </section>
  );
}

export function AiUnderstandingCard({ item }: { item: ClipItem }) {
  return (
    <GlassPanel variant="ai" className="ai-understanding-card">
      <SectionHeader
        title="内容分析"
        subtitle="自动生成"
        icon={<Sparkles className="h-4 w-4 text-[var(--accent)]" />}
      />
      <p className="relative z-[1] text-[14px] leading-6 text-[var(--text-main)]">{item.analysis.summary}</p>
      <div className="ai-facts">
        {item.analysis.insights.map((insight) => (
          <div key={insight.label}>
            <span>{insight.label}</span>
            <strong>{insight.value}</strong>
          </div>
        ))}
      </div>
    </GlassPanel>
  );
}

export function DetailPanel({
  item,
  loading,
  onPin,
  onDelete,
  editingTitle,
  editTitleVal,
  onEditTitleChange,
  onSaveTitle,
  onStartEditTitle,
  editingTags,
  editTagsVal,
  onEditTagsChange,
  onSaveTags,
  onStartEditTags,
}: {
  item: ClipItem;
  loading: boolean;
  onPin: () => void;
  onDelete: () => void;
  editingTitle: boolean;
  editTitleVal: string;
  onEditTitleChange: (v: string) => void;
  onSaveTitle: () => void;
  onStartEditTitle: () => void;
  editingTags: boolean;
  editTagsVal: string;
  onEditTagsChange: (v: string) => void;
  onSaveTags: () => void;
  onStartEditTags: () => void;
}) {
  const meta = typeMeta(item.type);
  const Icon = meta.icon;

  if (loading) {
    return (
      <section className="detail-panel">
        <div className="detail-skeleton">
          <Sparkles className="h-5 w-5 text-[var(--accent)]" />
          <div>
            <div className="text-[15px] font-semibold text-[var(--text-main)]">加载中...</div>
          </div>
        </div>
      </section>
    );
  }

  return (
    <section key={item.id} className="detail-panel detail-panel-enter">
      <div className="mx-auto max-w-[680px]">
        <div className="detail-header">
          <div>
            <div className="mb-3 flex items-center gap-2 text-[13px] text-[var(--text-secondary)]">
              <Icon className="h-4 w-4 text-[var(--accent)]" />
              {meta.label}
              {item.pinned && <span className="content-type-pill" style={{ color: "var(--accent)" }}>置顶</span>}
            </div>
            {editingTitle ? (
              <input autoFocus value={editTitleVal} onChange={(e) => onEditTitleChange(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") void onSaveTitle(); if (e.key === "Escape") onStartEditTitle(); }}
                onBlur={() => void onSaveTitle()}
                className="w-full rounded-lg border border-[var(--accent-border)] bg-[var(--bg-card)] px-3 py-1.5 text-[18px] font-semibold text-[var(--text-main)] outline-none" />
            ) : (
              <h1 className="text-[22px] font-semibold text-[var(--text-main)] cursor-pointer hover:text-[var(--accent)]" onClick={onStartEditTitle}>{item.title}</h1>
            )}
            {editingTags ? (
              <input autoFocus value={editTagsVal} onChange={(e) => onEditTagsChange(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") void onSaveTags(); if (e.key === "Escape") onStartEditTags(); }}
                onBlur={() => void onSaveTags()}
                placeholder="标签（逗号分隔）"
                className="mt-3 w-full rounded-lg border border-[var(--accent-border)] bg-[var(--bg-card)] px-3 py-1.5 text-[13px] text-[var(--text-main)] outline-none" />
            ) : (
              <div className="mt-3 flex flex-wrap gap-2 cursor-pointer" onClick={onStartEditTags}>
                {item.tags.map((tag) => <span key={tag} className="detail-tag">{tag}</span>)}
                <span className="detail-tag opacity-50">+ 编辑</span>
              </div>
            )}
            <p className="mt-3 text-[13px] text-[var(--text-muted)]">保存时间：{item.savedAt}</p>
          </div>
          <button type="button" className="icon-soft-button" aria-label="更多"><MoreHorizontal className="h-5 w-5" /></button>
        </div>

        <div className="mt-7">
          <SectionHeader
            title="原始内容"
            action={
              <button type="button" className="copy-chip" onClick={() => void navigator.clipboard.writeText(item.content)}>
                <Copy className="h-3.5 w-3.5" />复制
              </button>
            }
          />
          <div className="source-block"><pre>{item.content}</pre></div>
        </div>

        <AiUnderstandingCard item={item} />

        <div className="mt-7 flex flex-wrap gap-2 pb-4">
          <button type="button" className="detail-action" onClick={onPin}>
            <PenLine className="h-4 w-4" />{item.pinned ? "取消置顶" : "置顶"}
          </button>
          <button type="button" className="detail-action-danger" onClick={onDelete}>
            <Trash2 className="h-4 w-4" />删除
          </button>
        </div>
      </div>
    </section>
  );
}

const CATEGORY_QUERY_MAP: Record<string, string> = {
  "全部": "",
  "最近保存": "",
  "AI": "ai",
  "NAS": "nas",
  "代码": "code",
  "教程": "tutorial",
  "网址": "url http",
  "图片": "image png jpg",
};

export function MainLibrary({ onToast }: { onToast: (title: string, description?: string) => void }) {
  const [items, setItems] = useState<ClipItem[]>([]);
  const [selectedId, setSelectedId] = useState(0);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [query, setQuery] = useState("");
  const [activeCategory, setActiveCategory] = useState("全部");
  const [editingTitle, setEditingTitle] = useState(false);
  const [editTitleVal, setEditTitleVal] = useState("");
  const [editingTags, setEditingTags] = useState(false);
  const [editTagsVal, setEditTagsVal] = useState("");
  const debounceRef = useRef<number | null>(null);

  const loadItems = useCallback(async (q: string) => {
    try {
      const results = await invoke<Snippet[]>("search_snippets", { query: q });
      const mapped = results.map(snippetToClipItem);
      setItems(mapped);
      if (mapped.length > 0 && !mapped.find((i) => i.id === selectedId)) {
        setSelectedId(mapped[0].id);
      }
    } catch {
      setItems([]);
    }
  }, [selectedId]);

  useEffect(() => {
    void loadItems("");
    const win = getCurrentWindow();
    const blurTimer = { current: null as number | null };

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        void win.hide();
      }
    };
    window.addEventListener("keydown", onKeyDown);

    const unlisten = win.onFocusChanged((e) => {
      if (e.payload) {
        if (blurTimer.current) { window.clearTimeout(blurTimer.current); blurTimer.current = null; }
        void loadItems(query);
      } else {
        blurTimer.current = window.setTimeout(() => void win.hide(), 150);
      }
    });

    return () => {
      window.removeEventListener("keydown", onKeyDown);
      if (blurTimer.current) window.clearTimeout(blurTimer.current);
      void unlisten.then((fn) => fn());
    };
  }, [loadItems, query]);

  useEffect(() => {
    if (debounceRef.current) window.clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => void loadItems(query), 150);
    return () => { if (debounceRef.current) window.clearTimeout(debounceRef.current); };
  }, [query, loadItems]);

  const selected = items.find((i) => i.id === selectedId) ?? (items.length > 0 ? items[0] : null);

  const selectItem = (id: number) => {
    if (id === selectedId) return;
    setSelectedId(id);
    setLoadingDetail(true);
    window.setTimeout(() => setLoadingDetail(false), 300);
  };

  const handlePin = async () => {
    if (!selected) return;
    await invoke("toggle_pin", { id: selected.id });
    await loadItems(query);
    onToast(selected.pinned ? "已取消置顶" : "已置顶", selected.title);
  };

  const handleDelete = async () => {
    if (!selected) return;
    await invoke("delete_snippet", { id: selected.id });
    await loadItems(query);
    onToast("已删除", selected.title);
  };

  const startEditTitle = () => {
    if (!selected) return;
    setEditTitleVal(selected.title);
    setEditingTitle(true);
  };

  const saveEditTitle = async () => {
    if (!selected || !editTitleVal.trim()) { setEditingTitle(false); return; }
    try {
      await invoke("update_clip", { id: selected.id, title: editTitleVal.trim() });
      setEditingTitle(false);
      await loadItems(query);
      onToast("标题已更新");
    } catch { onToast("更新失败", "请稍后重试"); }
  };

  const startEditTags = () => {
    if (!selected) return;
    setEditTagsVal(selected.tags.join(", "));
    setEditingTags(true);
  };

  const saveEditTags = async () => {
    if (!selected) { setEditingTags(false); return; }
    try {
      await invoke("update_clip", { id: selected.id, tags: editTagsVal.trim() || null });
      setEditingTags(false);
      await loadItems(query);
      onToast("标签已更新");
    } catch { onToast("更新失败", "请稍后重试"); }
  };

  const handleExport = async () => {
    try {
      const path = await invoke<string>("export_markdown");
      onToast("导出成功", path);
    } catch { onToast("导出失败", "请稍后重试"); }
  };

  return (
    <div className="library-shell">
      <Sidebar active={activeCategory} onNavigate={(cat) => { setActiveCategory(cat); setQuery(CATEGORY_QUERY_MAP[cat] ?? cat); }} onSettings={() => onToast("设置功能", "即将上线")} onExport={() => void handleExport()} />
      <ContentList items={items} selectedId={selectedId} onSelect={selectItem} query={query} onQueryChange={setQuery} />
      {selected ? (
        <DetailPanel item={selected} loading={loadingDetail} onPin={() => void handlePin()} onDelete={() => void handleDelete()}
            editingTitle={editingTitle} editTitleVal={editTitleVal} onEditTitleChange={setEditTitleVal} onSaveTitle={() => void saveEditTitle()} onStartEditTitle={startEditTitle}
            editingTags={editingTags} editTagsVal={editTagsVal} onEditTagsChange={setEditTagsVal} onSaveTags={() => void saveEditTags()} onStartEditTags={startEditTags} />
      ) : (
        <section className="detail-panel flex items-center justify-center">
          <p className="text-[var(--text-muted)]">选择一条记录查看详情</p>
        </section>
      )}
    </div>
  );
}

function App() {
  const [label, setLabel] = useState("");
  const [toast, setToast] = useState<{ title: string; description?: string; show: boolean; tone?: ToastTone }>({
    title: "",
    description: "",
    show: false,
  });

  useEffect(() => {
    setLabel(getCurrentWindow().label);
  }, []);

  const showToast = (title: string, description?: string, tone: ToastTone = "success") => {
    setToast({ title, description, show: true, tone });
    window.setTimeout(() => setToast((c) => ({ ...c, show: false })), 2000);
  };

  return (
    <main className="app-root">
      <AmbientBackground />
      <div className="preview-area">
        {label === "save" && <CapturePopup onToast={showToast} />}
        {label === "search" && <MainLibrary onToast={showToast} />}
        {!label && (
          <div className="flex h-full items-center justify-center">
            <Loader2 className="h-5 w-5 animate-spin text-[var(--accent)]" />
          </div>
        )}
      </div>
      <Toast show={toast.show} title={toast.title} description={toast.description} tone={toast.tone} />
    </main>
  );
}

export default App;
