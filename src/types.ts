// 全局类型定义

export type ClipboardContentType = "code" | "url" | "prompt" | "text" | "image";

export interface ClipboardInsight {
  label: string;
  value: string;
}

export interface ClipboardAnalysis {
  title: string;
  type: ClipboardContentType;
  summary: string;
  tags: string[];
  insights: ClipboardInsight[];
  related: string[];
}

export interface Snippet {
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
  image_path?: string | null;
  image_dim_w?: number | null;
  image_dim_h?: number | null;
  ocr_status?: "pending" | "done" | "failed" | "skipped" | null;
}

export interface ClipItem {
  id: number;
  title: string;
  summary: string;
  content: string;
  tags: string[];
  time: string;
  savedAt: string;
  type: ClipboardContentType;
  pinned: boolean;
  image_path?: string | null;
  image_dim_w?: number | null;
  image_dim_h?: number | null;
  ocr_status?: string | null;
}

export interface OcrDoneInfo {
  text: string;
  source: string;
}

export interface OcrCapability {
  engine: string;
  available: boolean;
  python_detected: boolean;
  python_path: string | null;
  script_bundled: boolean;
  message: string | null;
}

export interface TagSummary {
  name: string;
  count: number;
}

export interface AiTagResult {
  tags: string[];
  summary: string;
}

export interface ImportResult {
  imported: number;
  path: string;
}

export type GlassVariant = "popup" | "window" | "card" | "ai";
export type ToastTone = "success" | "info" | "warning" | "error";

export interface ScreenOcrRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type WindowLabel = "capture" | "search" | "settings" | "screen_ocr";
