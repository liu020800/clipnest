// 内容分析: 类型识别 + 标题生成 + 标签建议
import type { ClipboardAnalysis, ClipboardContentType, ClipboardInsight } from "../types";

const PROMPT_KEYWORDS = [
  "请", "帮我", "帮我写", "写一个", "写一段", "解释", "翻译",
  "总结", "提炼", "改写", "重写", "润色", "建议", "推荐",
  "分析", "说明", "介绍一下", "什么是", "怎么", "如何",
  "扮演", "角色", "假如", "假设",
  "write", "explain", "translate", "summarize", "analyze",
  "rewrite", "describe", "introduce", "what is", "how to",
  "act as", "pretend", "imagine",
];

const CODE_PATTERNS: Array<{ lang: string; pattern: RegExp }> = [
  { lang: "rust", pattern: /\bfn\s+\w+\s*\(|\blet\s+mut\s|\bimpl\s+\w+/ },
  { lang: "python", pattern: /^def\s+\w+\s*\(|^from\s+\w+\s+import\s+|^import\s+\w+|\bprint\(/m },
  { lang: "javascript", pattern: /\bconst\s+\w+\s*=|=>\s*[{(]|\brequire\(|console\.(log|error)/ },
  { lang: "typescript", pattern: /:\s*(string|number|boolean|void|any)\b|interface\s+\w+\s*\{/ },
  { lang: "shell", pattern: /^(curl|ssh|docker|npm|npx|pnpm|yarn|pip|python|git|cd|ls|rm|mkdir|cp|mv)\b/m },
  { lang: "sql", pattern: /\b(SELECT|INSERT|UPDATE|DELETE|CREATE|ALTER|DROP)\b.*\b(FROM|INTO|TABLE|WHERE)\b/i },
  { lang: "yaml", pattern: /^[a-zA-Z_][\w-]*:\s*(\S|$)/m },
  { lang: "json", pattern: /^\s*\{[\s\S]*"[\w-]+":\s*/ },
  { lang: "html", pattern: /<\/?[a-z][\w-]*[\s>]/i },
  { lang: "css", pattern: /^[.#@]?[\w-]+\s*\{[\s\S]*?(?:color|background|margin|padding):/m },
  { lang: "ini", pattern: /^\[[\w.-]+\]\s*$/m },
  { lang: "nginx", pattern: /^(server|location|upstream|proxy_pass|listen)\s+/m },
];

function detectPrompt(text: string): boolean {
  const trimmed = text.trim();
  if (trimmed.length === 0) return false;
  if (trimmed.length > 400) return false;
  const lower = trimmed.toLowerCase();
  return PROMPT_KEYWORDS.some((kw) => lower.includes(kw.toLowerCase()));
}

function detectUrl(text: string): boolean {
  const t = text.trim();
  if (!/^https?:\/\//i.test(t)) return false;
  if (/\s/.test(t)) return false;
  try {
    new URL(t);
    return true;
  } catch {
    return false;
  }
}

function detectCodeLang(text: string): string | null {
  for (const { lang, pattern } of CODE_PATTERNS) {
    if (pattern.test(text)) return lang;
  }
  if (text.includes("\n") && /[{};]/.test(text) && /\b(function|class|return|if|else|for|while)\b/.test(text)) {
    return "code";
  }
  if (text.includes("\n") && /[{}();]/.test(text)) {
    return "code";
  }
  return null;
}

function summarizeUrl(url: string): string {
  try {
    const u = new URL(url);
    return u.hostname.replace(/^www\./, "");
  } catch {
    return url.slice(0, 28);
  }
}

function titleFromUrl(url: string): string {
  try {
    const u = new URL(url);
    return u.hostname.replace(/^www\./, "").slice(0, 28);
  } catch {
    return url.slice(0, 28);
  }
}

function titleFromCode(text: string, lang: string): string {
  const lines = text.split("\n");
  for (const raw of lines) {
    const line = raw.trim();
    if (!line || line.startsWith("//") || line.startsWith("#") || line.startsWith("/*")) {
      continue;
    }
    if (lang === "shell") {
      const m = line.match(/^(?:curl|ssh|docker|npm|npx|pnpm|yarn|pip|python|git)\s+([\w-]+)/);
      if (m) return `${line.split(/\s+/)[0]} ${m[1]}`.slice(0, 28);
    }
    if (lang === "sql") {
      const m = line.match(/\b(?:FROM|INTO|UPDATE|TABLE)\s+`?(\w+)`?/i);
      if (m) return `${line.match(/^\w+/)![0]} ${m[1]}`.slice(0, 28);
    }
    if (lang === "rust") {
      const m = line.match(/\bfn\s+(\w+)/);
      if (m) return `fn ${m[1]}`.slice(0, 28);
    }
    if (lang === "python") {
      const m = line.match(/\bdef\s+(\w+)/);
      if (m) return `def ${m[1]}`.slice(0, 28);
    }
    if (lang === "javascript" || lang === "typescript") {
      const m = line.match(/\b(?:function|const|let|var)\s+(\w+)/);
      if (m) return `${m[1]}`.slice(0, 28);
    }
    return line.slice(0, 28);
  }
  return lines[0]?.trim().slice(0, 28) || "代码片段";
}

function titleFromPrompt(text: string): string {
  const first = text.split(/[。.!?！？\n]/)[0].trim();
  return (first || "Prompt").slice(0, 28);
}

function titleFromText(text: string): string {
  const first = text.split(/[。.!?！？\n]/)[0].trim();
  return (first || "未命名").slice(0, 28);
}

export function analyzeClipboardContent(content: string): ClipboardAnalysis {
  const trimmed = content.trim();
  let type: ClipboardContentType = "text";
  let title = "";
  let tags: string[] = [];
  const insights: ClipboardInsight[] = [];

  if (detectUrl(trimmed)) {
    type = "url";
    title = titleFromUrl(trimmed);
    tags = ["网址"];
    const summary = summarizeUrl(trimmed);
    insights.push({ label: "域名", value: summary });
    return {
      title,
      type,
      summary: trimmed.length > 120 ? trimmed.slice(0, 120) + "..." : trimmed,
      tags,
      insights,
      related: [],
    };
  }

  const lang = detectCodeLang(trimmed);
  if (lang) {
    type = "code";
    title = titleFromCode(trimmed, lang);
    tags = ["代码", lang];
    insights.push({ label: "语言", value: lang });
    return {
      title,
      type,
      summary: trimmed.split("\n")[0].trim().slice(0, 120),
      tags,
      insights,
      related: [],
    };
  }

  if (detectPrompt(trimmed)) {
    type = "prompt";
    title = titleFromPrompt(trimmed);
    tags = ["Prompt"];
    return {
      title,
      type,
      summary: trimmed.length > 120 ? trimmed.slice(0, 120) + "..." : trimmed,
      tags,
      insights,
      related: [],
    };
  }

  type = "text";
  title = titleFromText(trimmed);
  tags = ["文本"];
  return {
    title,
    type,
    summary: trimmed.length > 120 ? trimmed.slice(0, 120) + "..." : trimmed,
    tags,
    insights,
    related: [],
  };
}
