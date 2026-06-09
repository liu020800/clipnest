import { useCallback, useEffect, useRef, useState } from "react";
import { api, safeOnFocusChanged } from "../lib/api";
import { Sidebar, categoryToFilter, type CategoryKey } from "../components/Sidebar";
import { ContentList } from "../components/ContentList";
import { DetailPanel } from "../components/DetailPanel";
import { useKeyboardNavigation } from "../hooks/useKeyboardNavigation";
import { useSettings } from "../hooks/useSettings";
import { snippetToClipItem } from "../lib/mappers";
import type { ClipItem } from "../types";

export function SearchWindow({
  onToast,
}: {
  onToast: (title: string, description?: string, tone?: "success" | "info" | "warning" | "error") => void;
}) {
  const [items, setItems] = useState<ClipItem[]>([]);
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<CategoryKey>("all");
  const [selectedId, setSelectedId] = useState(0);
  const [loading, setLoading] = useState(false);
  const debounceRef = useRef<number | null>(null);
  const queryRef = useRef("");
  const categoryRef = useRef<CategoryKey>("all");

  const settings = useSettings();
  const debounceMs = settings.getNum("search_debounce_ms", 150);
  const searchLimit = settings.getNum("search_limit", 50);

  const load = useCallback(
    async (q: string, cat: CategoryKey) => {
      setLoading(true);
      try {
        const filter = categoryToFilter(cat);
        const baseQuery = q;
        const finalQuery = filter.query !== "" ? filter.query : baseQuery;
        const results = await api.listSnippets({
          query: finalQuery,
          filterKind: filter.filterKind,
          filterValue: filter.filterValue,
          limit: searchLimit,
        });
        setItems(results.map(snippetToClipItem));
        if (results.length > 0 && !results.find((i) => i.id === selectedId)) {
          setSelectedId(results[0].id);
        } else if (results.length === 0) {
          setSelectedId(0);
        }
      } catch (e) {
        console.error("load failed", e);
        setItems([]);
      } finally {
        setLoading(false);
      }
    },
    [searchLimit, selectedId],
  );

  useEffect(() => {
    queryRef.current = query;
    categoryRef.current = category;
  });

  const settingsRef = useRef(settings);
  useEffect(() => {
    settingsRef.current = settings;
  });

  const blurTimerRef = useRef<number>(0);

  useEffect(() => {
    void load("", category);

    const unlisten = safeOnFocusChanged((focused) => {
      if (focused) {
        void load(queryRef.current, categoryRef.current);
      } else {
        const s = settingsRef.current;
        if (!s.getBool("auto_close_on_blur", true)) return;
        const delay = s.getNum("auto_close_delay_ms", 150);
        if (blurTimerRef.current) window.clearTimeout(blurTimerRef.current);
        if (delay <= 0) {
          void api.hideCurrentWindow();
        } else {
          blurTimerRef.current = window.setTimeout(() => {
            void api.hideCurrentWindow();
          }, delay);
        }
      }
    });
    return () => {
      if (blurTimerRef.current) window.clearTimeout(blurTimerRef.current);
      blurTimerRef.current = 0;
      unlisten();
    };
  }, [load, category]);

  useEffect(() => {
    if (debounceRef.current) window.clearTimeout(debounceRef.current);
    debounceRef.current = window.setTimeout(() => {
      void load(query, category);
    }, debounceMs);
    return () => {
      if (debounceRef.current) window.clearTimeout(debounceRef.current);
    };
  }, [query, category, debounceMs, load]);

  const selected = items.find((i) => i.id === selectedId) ?? items[0] ?? null;

  const copySelected = useCallback(
    async (closeAfter: boolean) => {
      if (!selected) return;
      try {
        await api.copyToClipboard(selected.content);
        onToast("已复制", selected.title, "success");
        if (closeAfter) {
          window.setTimeout(() => void api.hideCurrentWindow(), 200);
        }
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        onToast("复制失败", msg, "error");
      }
    },
    [selected, onToast],
  );

  const handleNavigate = (cat: CategoryKey) => {
    setCategory(cat);
  };

  const handleSettings = () => {
    void api.openSettings();
  };

  const handleSelect = (id: number) => setSelectedId(id);

  const refreshAfterMutation = useCallback(() => {
    void load(queryRef.current, categoryRef.current);
  }, [load]);

  const onEnter = () => void copySelected(true);
  const onCopyKeepOpen = () => void copySelected(false);
  const onEscape = () => void api.hideCurrentWindow();

  useKeyboardNavigation({
    enabled: true,
    count: items.length,
    selectedIndex: selected ? items.findIndex((i) => i.id === selected.id) : 0,
    setSelectedIndex: (i) => {
      const it = items[i];
      if (it) setSelectedId(it.id);
    },
    onEnter,
    onCopyKeepOpen,
    onEscape,
  });

  return (
    <div className="library-shell">
      <Sidebar
        active={category}
        onNavigate={handleNavigate}
        onSettings={handleSettings}
      />
      <ContentList
        items={items}
        selectedId={selectedId}
        onSelect={handleSelect}
        query={query}
        onQueryChange={setQuery}
      />
      <DetailPanel
        item={selected}
        loading={loading}
        query={query}
        onAfterMutation={refreshAfterMutation}
        onCopy={(text, label) => {
          void api.copyToClipboard(text).then(() => {
            onToast(label, undefined, "success");
          }).catch((e) => {
            onToast("复制失败", String(e), "error");
          });
        }}
      />
    </div>
  );
}
