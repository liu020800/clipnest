import { useEffect, useState } from "react";
import { Archive, Clipboard, Pin, Code2, Globe2, FileText, Sparkles, Tag as TagIcon, Settings } from "lucide-react";
import { api } from "../lib/api";
import { cn } from "../lib/utils";
import type { TagSummary } from "../types";

export type CategoryKey =
  | "all"
  | "recent"
  | "pinned"
  | "code"
  | "url"
  | "prompt"
  | "text"
  | { tag: string };

export function categoryToFilter(c: CategoryKey): { query: string; filterKind?: "pinned" | "recent" | "type" | "tag"; filterValue?: string } {
  if (typeof c !== "string") {
    return { query: "", filterKind: "tag", filterValue: c.tag };
  }
  switch (c) {
    case "all":
      return { query: "" };
    case "recent":
      return { query: "", filterKind: "recent" };
    case "pinned":
      return { query: "", filterKind: "pinned" };
    case "code":
    case "url":
    case "prompt":
    case "text":
      return { query: "", filterKind: "type", filterValue: c };
    default:
      return { query: c };
  }
}

const STATIC_GROUPS: Array<Array<{ key: CategoryKey; label: string; Icon: typeof Archive }>> = [
  [
    { key: "all", label: "全部", Icon: Archive },
    { key: "recent", label: "最近", Icon: Clipboard },
    { key: "pinned", label: "置顶", Icon: Pin },
  ],
  [
    { key: "code", label: "代码", Icon: Code2 },
    { key: "url", label: "网址", Icon: Globe2 },
    { key: "prompt", label: "Prompt", Icon: Sparkles },
    { key: "text", label: "文本", Icon: FileText },
  ],
];

function isCategoryMatch(a: CategoryKey, b: CategoryKey): boolean {
  if (typeof a === "string" && typeof b === "string") return a === b;
  if (typeof a === "object" && typeof b === "object") return a.tag === b.tag;
  return false;
}

export function Sidebar({
  active,
  onNavigate,
  onSettings,
}: {
  active: CategoryKey;
  onNavigate: (c: CategoryKey) => void;
  onSettings?: () => void;
}) {
  const [tags, setTags] = useState<TagSummary[]>([]);

  useEffect(() => {
    void api.listTags().then(setTags).catch(() => setTags([]));
  }, []);

  const renderItem = (key: CategoryKey, label: string, Icon: typeof Archive) => (
    <button
      key={typeof key === "string" ? key : `tag:${key.tag}`}
      type="button"
      onClick={() => onNavigate(key)}
      className={cn("sidebar-item", isCategoryMatch(active, key) && "sidebar-item-active")}
    >
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
            <div className="text-[11px] text-[var(--text-muted)]">v1.1.0-beta.1 · 本地知识捕获</div>
          </div>
        </div>
      </div>

      <nav className="mt-5 space-y-3">
        {STATIC_GROUPS.map((group, gi) => (
          <div key={gi} className="sidebar-group">
            {group.map((g) => renderItem(g.key, g.label, g.Icon))}
          </div>
        ))}
        {tags.length > 0 && (
          <div className="sidebar-group">
            <div className="sidebar-group-title">
              <TagIcon className="h-3 w-3" />
              标签
            </div>
            {tags.slice(0, 12).map((t) => renderItem({ tag: t.name }, t.name, TagIcon))}
          </div>
        )}
      </nav>

      <div className="mt-auto space-y-1 pt-5">
        {onSettings && (
          <button type="button" className="sidebar-item" onClick={onSettings}>
            <Settings className="h-[15px] w-[15px]" />
            设置
          </button>
        )}
      </div>
    </aside>
  );
}
