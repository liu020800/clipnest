import React from "react";
import { Search } from "lucide-react";
import { ContentCard } from "./ContentCard";
import type { ClipItem } from "../types";

export function ContentList({
  items,
  selectedId,
  onSelect,
  query,
  onQueryChange,
  listRef,
}: {
  items: ClipItem[];
  selectedId: number;
  onSelect: (id: number) => void;
  query: string;
  onQueryChange: (q: string) => void;
  listRef?: React.RefObject<HTMLDivElement>;
}) {
  return (
    <section className="content-list">
      <div className="content-list-top">
        <div className="search-input-shell">
          <Search className="h-4 w-4 text-[var(--text-muted)]" />
          <input
            autoFocus
            value={query}
            onChange={(e) => onQueryChange(e.target.value)}
            placeholder="搜索知识库... (#docker 支持标签过滤)"
            className="min-w-0 flex-1 bg-transparent text-[14px] text-[var(--text-main)] outline-none placeholder:text-[var(--text-muted)]"
          />
        </div>
      </div>
      <div ref={listRef} className="min-h-0 flex-1 space-y-2 overflow-y-auto p-3">
        {items.length === 0 && (
          <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
            <p className="text-sm text-[var(--text-secondary)]">
              {query ? "无匹配结果" : "暂无保存内容"}
            </p>
            <p className="text-[12px] text-[var(--text-muted)]">
              {query ? "试试更短的关键词" : "Ctrl+Shift+S 保存剪贴板内容"}
            </p>
          </div>
        )}
        {items.map((item) => (
          <ContentCard
            key={item.id}
            item={item}
            selected={item.id === selectedId}
            onClick={() => onSelect(item.id)}
            query={query}
          />
        ))}
      </div>
    </section>
  );
}
