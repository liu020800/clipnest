import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import SnippetCard from "./SnippetCard";

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

function SearchPanel() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Snippet[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceRef = useRef<number | null>(null);
  const blurTimerRef = useRef<number | null>(null);

  const search = useCallback(async (q: string) => {
    setLoading(true);
    setError("");

    try {
      const res = await invoke<Snippet[]>("search_snippets", { query: q });
      setResults(res);
      setSelectedIndex(0);
    } catch {
      setResults([]);
      setError("搜索失败，请检查本地数据库是否可用。");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const win = getCurrentWindow();
    const timer = window.setTimeout(() => {
      void win.setFocus();
      inputRef.current?.focus();
    }, 50);

    void search("");

    const unlistenPromise = win.onFocusChanged((event) => {
      if (event.payload) {
        if (blurTimerRef.current) {
          window.clearTimeout(blurTimerRef.current);
          blurTimerRef.current = null;
        }
        window.setTimeout(() => inputRef.current?.focus(), 60);
      } else {
        blurTimerRef.current = window.setTimeout(() => void win.hide(), 120);
      }
    });

    return () => {
      window.clearTimeout(timer);
      void unlistenPromise.then((unlisten) => unlisten());
    };
  }, [search]);

  useEffect(() => {
    if (debounceRef.current) {
      window.clearTimeout(debounceRef.current);
    }

    debounceRef.current = window.setTimeout(() => {
      void search(query);
    }, 120);

    return () => {
      if (debounceRef.current) {
        window.clearTimeout(debounceRef.current);
      }
    };
  }, [query, search]);

  const copyAndClose = async (content: string) => {
    try {
      await invoke("copy_to_clipboard", { content });
      await getCurrentWindow().hide();
    } catch {
      setError("复制失败，请稍后再试。");
    }
  };

  const handleKeyDown = async (e: React.KeyboardEvent<HTMLInputElement>) => {
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, Math.max(results.length - 1, 0)));
        break;
      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
        break;
      case "Enter":
        e.preventDefault();
        if (results[selectedIndex]) {
          await copyAndClose(results[selectedIndex].content);
        }
        break;
      case "Escape":
        e.preventDefault();
        await getCurrentWindow().hide();
        break;
    }
  };

  const handleMouseDown = () => {
    if (blurTimerRef.current) {
      window.clearTimeout(blurTimerRef.current);
      blurTimerRef.current = null;
    }
  };

  return (
    <div className="window-shell h-screen w-screen p-2" onMouseDown={handleMouseDown}>
      <div className="glass-panel flex h-full flex-col overflow-hidden rounded-[20px]">
        <div className="flex items-center gap-2 border-b border-[var(--border-soft)] px-3 py-2">
          <svg className="h-4 w-4 shrink-0 text-[var(--text-dim)]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.8}
              d="M21 21l-4.35-4.35m1.35-5.15a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="搜索..."
            className="min-w-0 flex-1 bg-transparent text-sm text-[var(--text-primary)] outline-none placeholder-[var(--text-dim)]"
          />
          <span className="shrink-0 text-[10px] text-[var(--text-dim)]">
            {loading ? "..." : `${results.length} 条`}
          </span>
          <span className="shrink-0 text-[10px] text-[var(--text-dim)] opacity-50">↑↓选择 ↵复制 ⎋隐藏</span>
        </div>

        <div className="min-h-0 flex-1 overflow-hidden">
          <div className="h-full overflow-y-auto px-2 py-2">
            {error && (
              <div className="mb-2 rounded-lg border border-[rgba(255,120,120,0.2)] bg-[rgba(120,35,35,0.22)] px-2.5 py-1.5 text-[11px] text-[var(--danger)]">
                {error}
              </div>
            )}

            {results.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
                <p className="text-sm text-[var(--text-secondary)]">
                  {query ? "无匹配" : "暂无内容"}
                </p>
                <p className="text-[11px] text-[var(--text-dim)]">
                  {query ? "试试更短的关键词或拼音" : "Alt+W 保存剪贴板内容"}
                </p>
              </div>
            ) : (
              <div className="space-y-1.5">
                {results.map((snippet, index) => (
                  <SnippetCard
                    key={snippet.id}
                    snippet={snippet}
                    selected={index === selectedIndex}
                    query={query}
                    onClick={() => void copyAndClose(snippet.content)}
                    onPin={async () => {
                      await invoke("toggle_pin", { id: snippet.id });
                      await search(query);
                    }}
                    onDelete={async () => {
                      await invoke("delete_snippet", { id: snippet.id });
                      await search(query);
                    }}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

export default SearchPanel;
