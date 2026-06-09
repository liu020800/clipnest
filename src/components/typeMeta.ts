import { Code2, Link2, FileText, Sparkles, ImageIcon } from "lucide-react";
import type { ClipboardContentType } from "../types";

export function typeMeta(type: ClipboardContentType) {
  if (type === "code") return { label: "代码", icon: Code2 };
  if (type === "url") return { label: "网址", icon: Link2 };
  if (type === "prompt") return { label: "Prompt", icon: Sparkles };
  if (type === "image") return { label: "图片", icon: ImageIcon };
  return { label: "文本", icon: FileText };
}
