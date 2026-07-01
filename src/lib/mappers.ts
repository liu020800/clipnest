// Snippet → ClipItem 映射
import type { ClipboardContentType, ClipItem, Snippet } from "../types";
import { formatTime } from "./utils";
import { analyzeClipboardContent } from "./analyze";

const ALLOWED_TYPES: ReadonlySet<string> = new Set<ClipboardContentType>([
  "code",
  "url",
  "prompt",
  "text",
  "image",
]);

function normalizeType(
  raw: string | null | undefined,
  fallback: ClipboardContentType,
): ClipboardContentType {
  if (raw && ALLOWED_TYPES.has(raw)) {
    return raw as ClipboardContentType;
  }
  return fallback;
}

export function snippetToClipItem(s: Snippet): ClipItem {
  const analysis = s.type === "image"
    ? { title: s.title, type: "image" as const, summary: s.content || "图片片段", tags: [], insights: [], related: [] }
    : analyzeClipboardContent(s.content);
  const tags = s.tags
    ? s.tags.split(",").map((t) => t.trim()).filter(Boolean)
    : [];
  return {
    id: s.id,
    title: s.title,
    summary: analysis.summary,
    content: s.content,
    tags,
    time: formatTime(s.created_at),
    savedAt: s.created_at,
    type: normalizeType(s.type, analysis.type),
    pinned: s.pinned,
    image_path: s.image_path ?? null,
    image_dim_w: s.image_dim_w ?? null,
    image_dim_h: s.image_dim_h ?? null,
    ocr_status: s.ocr_status ?? null,
  };
}
