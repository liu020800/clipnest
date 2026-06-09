// 通用工具
import { type ReactNode } from "react";

export function cn(...classes: Array<string | false | undefined | null>): string {
  return classes.filter(Boolean).join(" ");
}

export function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function formatTime(raw: string): string {
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

export function highlightMatches(
  text: string,
  query: string,
  React: typeof import("react"),
): ReactNode {
  if (!query.trim()) return text;
  const terms = query
    .split(/\s+/)
    .map((t) => t.trim())
    .filter((t) => t.length > 0)
    .map(escapeRegExp);
  if (terms.length === 0) return text;
  const re = new RegExp(`(${terms.join("|")})`, "gi");
  const parts = text.split(re);
  return parts.map((part, i) => {
    if (i % 2 === 1) {
      return React.createElement("mark", { key: i, className: "match-highlight" }, part);
    }
    return React.createElement("span", { key: i }, part);
  });
}

export function parseDuplicateError(msg: string): { id: number; title: string } | null {
  if (!msg.startsWith("DUPLICATE::")) return null;
  try {
    const json = msg.slice("DUPLICATE::".length);
    const parsed = JSON.parse(json);
    if (typeof parsed.id === "number" && typeof parsed.title === "string") {
      return { id: parsed.id, title: parsed.title };
    }
  } catch {
    // fall through
  }
  return null;
}

export function classifyClipboardType(content: string): "url" | "code" | "prompt" | "text" {
  const t = content.trim();
  if (/^https?:\/\/\S+$/i.test(t)) return "url";
  if (/\n/.test(t) && /[{};]/.test(t) && /\b(function|class|return|if|else|for|while)\b/.test(t)) {
    return "code";
  }
  if (/^(curl|ssh|docker|npm|npx|pnpm|yarn|pip|python|git)\s+/m.test(t)) {
    return "code";
  }
  if (t.length < 400 && /(请|帮我|写|解释|翻译|总结|分析|扮演|act as|explain|translate|summarize)/i.test(t)) {
    return "prompt";
  }
  return "text";
}
