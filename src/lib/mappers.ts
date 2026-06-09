// Snippet → ClipItem 映射
import type { ClipItem, Snippet } from "../types";
import { formatTime } from "./utils";
import { analyzeClipboardContent } from "./analyze";

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
    type: (s.type as ClipItem["type"]) ?? analysis.type,
    pinned: s.pinned,
    image_path: s.image_path ?? null,
    image_dim_w: s.image_dim_w ?? null,
    image_dim_h: s.image_dim_h ?? null,
    ocr_status: s.ocr_status ?? null,
  };
}
