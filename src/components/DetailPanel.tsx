import React, { useState, useEffect } from "react";
import { Loader2, Copy, Pin, Trash2, Pencil, Check, X, ExternalLink, ScanText } from "lucide-react";
import { api } from "../lib/api";
import { highlightMatches, cn } from "../lib/utils";
import { ImagePreview } from "./ImagePreview";
import { typeMeta } from "./typeMeta";
import type { ClipItem } from "../types";

function formatDateTime(raw: string): string {
  const d = new Date(raw.replace(" ", "T"));
  if (Number.isNaN(d.getTime())) return raw;
  return new Intl.DateTimeFormat("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(d);
}

function renderContent(content: string, type: string, query: string) {
  if (type === "url") {
    return (
      <a
        href={content}
        target="_blank"
        rel="noreferrer"
        className="detail-url"
      >
        <ExternalLink className="h-3.5 w-3.5" />
        {highlightMatches(content, query, React)}
      </a>
    );
  }
  if (type === "code" || type === "prompt") {
    return (
      <pre className="detail-code">
        <code>{highlightMatches(content, query, React)}</code>
      </pre>
    );
  }
  return (
    <pre className="detail-text">
      {highlightMatches(content, query, React)}
    </pre>
  );
}

export function DetailPanel({
  item,
  loading,
  query,
  onAfterMutation,
  onCopy,
}: {
  item: ClipItem | null;
  loading: boolean;
  query: string;
  onAfterMutation: () => void;
  onCopy: (text: string, label: string) => void;
}) {
  const [editingTitle, setEditingTitle] = useState(false);
  const [editingTags, setEditingTags] = useState(false);
  const [titleVal, setTitleVal] = useState("");
  const [tagsVal, setTagsVal] = useState("");

  useEffect(() => {
    setEditingTitle(false);
    setEditingTags(false);
    if (item) {
      setTitleVal(item.title);
      setTagsVal(item.tags.join(", "));
    }
  }, [item?.id]);

  if (!item) {
    return (
      <section className="detail-panel flex items-center justify-center">
        <p className="text-[var(--text-muted)]">选择一条记录查看详情</p>
      </section>
    );
  }

  if (loading) {
    return (
      <section className="detail-panel">
        <div className="detail-skeleton">
          <Loader2 className="h-5 w-5 animate-spin text-[var(--accent)]" />
          <div className="text-[15px] font-semibold text-[var(--text-main)]">加载中...</div>
        </div>
      </section>
    );
  }

  const meta = typeMeta(item.type);
  const Icon = meta.icon;

  const commitTitle = async () => {
    const v = titleVal.trim();
    if (!v || v === item.title) {
      setEditingTitle(false);
      return;
    }
    try {
      await api.updateSnippet(item.id, v, null, undefined);
      onAfterMutation();
    } catch (e) {
      console.error("update title failed", e);
    }
    setEditingTitle(false);
  };

  const commitTags = async () => {
    const v = tagsVal.trim();
    try {
      await api.updateSnippet(item.id, undefined, v || null, undefined);
      onAfterMutation();
    } catch (e) {
      console.error("update tags failed", e);
    }
    setEditingTags(false);
  };

  const handlePin = async () => {
    try {
      await api.togglePin(item.id);
      onAfterMutation();
    } catch (e) {
      console.error("toggle pin failed", e);
    }
  };

  const handleDelete = async () => {
    if (!window.confirm(`确定删除「${item.title}」?`)) return;
    try {
      await api.deleteSnippet(item.id);
      onAfterMutation();
    } catch (e) {
      console.error("delete failed", e);
    }
  };

  return (
    <section className="detail-panel">
      <div className="detail-inner">
        <div className="detail-meta-row">
          <div className="flex items-center gap-2 text-[12px] text-[var(--text-secondary)]">
            <Icon className="h-3.5 w-3.5 text-[var(--accent)]" />
            {meta.label}
            {item.pinned && <span className="detail-pill" style={{ color: "var(--accent)" }}>置顶</span>}
          </div>
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              onClick={() => onCopy(item.content, "已复制内容")}
              className="detail-action"
              title="复制全文 (Ctrl+C)"
            >
              <Copy className="h-3.5 w-3.5" />
              复制
            </button>
            <button
              type="button"
              onClick={handlePin}
              className={cn("detail-action", item.pinned && "detail-action-active")}
              title="置顶/取消置顶"
            >
              <Pin className="h-3.5 w-3.5" />
              {item.pinned ? "取消置顶" : "置顶"}
            </button>
            <button
              type="button"
              onClick={handleDelete}
              className="detail-action detail-action-danger"
              title="删除"
            >
              <Trash2 className="h-3.5 w-3.5" />
              删除
            </button>
          </div>
        </div>

        {editingTitle ? (
          <div className="detail-edit-row">
            <input
              autoFocus
              value={titleVal}
              onChange={(e) => setTitleVal(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void commitTitle();
                if (e.key === "Escape") setEditingTitle(false);
              }}
              className="detail-input"
            />
            <button onClick={() => void commitTitle()} className="detail-edit-btn">
              <Check className="h-3.5 w-3.5" />
            </button>
            <button onClick={() => setEditingTitle(false)} className="detail-edit-btn">
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        ) : (
          <h1
            className="detail-title"
            onClick={() => {
              setTitleVal(item.title);
              setEditingTitle(true);
            }}
            title="点击编辑标题"
          >
            {highlightMatches(item.title, query, React)}
            <Pencil className="h-3.5 w-3.5 opacity-0 group-hover:opacity-100" />
          </h1>
        )}

        {editingTags ? (
          <div className="detail-edit-row">
            <input
              autoFocus
              value={tagsVal}
              onChange={(e) => setTagsVal(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void commitTags();
                if (e.key === "Escape") setEditingTags(false);
              }}
              placeholder="逗号分隔,例如: docker,部署"
              className="detail-input"
            />
            <button onClick={() => void commitTags()} className="detail-edit-btn">
              <Check className="h-3.5 w-3.5" />
            </button>
            <button onClick={() => setEditingTags(false)} className="detail-edit-btn">
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        ) : (
          <div className="detail-tags" onClick={() => setEditingTags(true)} title="点击编辑标签">
            {item.tags.length === 0 ? (
              <span className="text-[12px] text-[var(--text-muted)] italic">点击添加标签...</span>
            ) : (
              item.tags.map((tag) => (
                <span key={tag} className="tag-badge tag-badge-ai">
                  {highlightMatches(tag, query, React)}
                </span>
              ))
            )}
          </div>
        )}

        <div className="detail-content">
          {renderContent(item.content, item.type, query)}
        </div>

        {item.image_path && (
          <div className="detail-image">
            <div className="detail-image-header">
              <span className="text-[12px] text-[var(--text-secondary)]">原图</span>
              {item.ocr_status && (
                <span
                  className={cn(
                    "detail-pill",
                    item.ocr_status === "done" && "text-[var(--success)]",
                    item.ocr_status === "failed" && "text-[var(--danger)]",
                    item.ocr_status === "skipped" && "text-[var(--text-muted)]",
                  )}
                >
                  <ScanText className="h-3 w-3" />
                  {item.ocr_status === "done" ? "OCR 已完成" : item.ocr_status === "failed" ? "OCR 失败" : item.ocr_status === "skipped" ? "OCR 已跳过" : "OCR 待处理"}
                </span>
              )}
            </div>
            <ImagePreview relPath={item.image_path} alt={item.title} />
            {item.image_dim_w && item.image_dim_h && (
              <div className="text-[11px] text-[var(--text-muted)]">
                {item.image_dim_w} × {item.image_dim_h}
              </div>
            )}
          </div>
        )}

        <div className="detail-times">
          <div>
            <span className="text-[var(--text-muted)]">创建</span> {formatDateTime(item.savedAt)}
          </div>
        </div>
      </div>
    </section>
  );
}
