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

interface Props {
  snippet: Snippet;
  selected: boolean;
  query: string;
  onClick: () => void;
  onPin: () => void;
  onDelete: () => void;
}

function formatSnippetDate(raw: string) {
  const normalized = raw.replace(" ", "T");
  const date = new Date(normalized);

  if (Number.isNaN(date.getTime())) {
    return raw;
  }

  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function escapeRegExp(value: string) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function normalizeQueryTerms(query: string) {
  return query
    .split(/\s+/)
    .map((term) => term.trim().replace(/^#/, "").replace(/[.*+?^${}()|[\]\\]/g, ""))
    .filter(Boolean);
}

function highlightText(text: string, query: string) {
  const terms = normalizeQueryTerms(query);
  if (!terms.length) {
    return text;
  }

  const pattern = new RegExp(`(${terms.map(escapeRegExp).join("|")})`, "gi");
  const segments = text.split(pattern);

  return segments.map((segment, index) => {
    const matched = terms.some((term) => segment.toLowerCase() === term.toLowerCase());
    if (!matched) {
      return <span key={`${segment}-${index}`}>{segment}</span>;
    }

    return (
      <mark key={`${segment}-${index}`} className="match-highlight">
        {segment}
      </mark>
    );
  });
}

function detectPinyinHint(snippet: Snippet, query: string) {
  const normalizedQuery = query.trim().toLowerCase().replace(/^#/, "");
  if (!normalizedQuery) {
    return "";
  }

  const titleLower = snippet.title.toLowerCase();
  const contentLower = snippet.content.toLowerCase();
  const tagsLower = snippet.tags?.toLowerCase() ?? "";

  const directHit =
    titleLower.includes(normalizedQuery) ||
    contentLower.includes(normalizedQuery) ||
    tagsLower.includes(normalizedQuery);

  if (directHit) {
    return "";
  }

  if (snippet.pinyin.toLowerCase().includes(normalizedQuery)) {
    return "拼音命中";
  }

  return "";
}

function SnippetCard({ snippet, selected, query, onClick, onPin, onDelete }: Props) {
  const pinyinHint = detectPinyinHint(snippet, query);

  return (
    <div
      onClick={onClick}
      className={`snippet-card ${selected ? "snippet-card-selected" : ""}`}
    >
      <div className="flex items-start gap-2.5">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            {snippet.pinned && (
              <span className="rounded-full bg-[rgba(198,173,136,0.15)] px-2 py-0.5 text-[10px] font-medium uppercase tracking-[0.16em] text-[var(--accent)]">
                置顶
              </span>
            )}
            {pinyinHint && <span className="meta-chip">{pinyinHint}</span>}
            <h3 className="truncate text-sm font-semibold text-[var(--text-primary)]">
              {highlightText(snippet.title, query)}
            </h3>
          </div>

          <p className="mt-1.5 line-clamp-3 text-xs leading-5 text-[var(--text-secondary)]">
            {highlightText(snippet.content.slice(0, 140), query)}
          </p>

          <div className="mt-2 flex flex-wrap items-center gap-2">
            {snippet.tags
              ?.split(",")
              .map((tag) => tag.trim())
              .filter(Boolean)
              .map((tag) => (
                <span key={tag} className="tag-chip">
                  #{highlightText(tag, query)}
                </span>
              ))}

            {snippet.type && (
              <span className="meta-chip">
                {snippet.type === "url"
                  ? "链接"
                  : snippet.type === "code"
                    ? "代码"
                    : "文本"}
              </span>
            )}

            <span className="ml-auto text-[11px] text-[var(--text-dim)]">
              {formatSnippetDate(snippet.created_at)}
            </span>
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1 self-start">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onPin();
            }}
            className="icon-button"
            title="固定 / 取消固定"
          >
            <svg
              className="h-4 w-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.8}
                d="M7 4h10l-1.5 6 2.5 2.5v1H13V20l-1 1-1-1v-6.5H6v-1L8.5 10 7 4z"
              />
            </svg>
          </button>

          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onDelete();
            }}
            className="icon-button hover:text-[var(--danger)]"
            title="删除"
          >
            <svg
              className="h-4 w-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.8}
                d="M19 7H5m3 0V5a1 1 0 011-1h6a1 1 0 011 1v2m-8 0l.7 11.2A2 2 0 0010.7 20h2.6a2 2 0 001.99-1.8L16 7M10 11v5m4-5v5"
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}

export default SnippetCard;
