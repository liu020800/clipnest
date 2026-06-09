// 片段列表管理: 加载、过滤、CRUD
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "../lib/api";
import { snippetToClipItem } from "../lib/mappers";
import type { ClipItem } from "../types";

export type FilterKind = "all" | "recent" | "pinned" | "code" | "url" | "prompt" | "text" | { tag: string };

export function useSnippets() {
  const [items, setItems] = useState<ClipItem[]>([]);
  const [loading, setLoading] = useState(false);
  const queryRef = useRef("");

  const load = useCallback(
    async (query: string, kind: FilterKind = "all") => {
      setLoading(true);
      try {
        let filterKind: "pinned" | "recent" | "type" | "tag" | undefined;
        let filterValue: string | undefined;
        let baseQuery = query;

        if (kind === "all") {
          // no extra filter
        } else if (kind === "recent") {
          filterKind = "recent";
        } else if (kind === "pinned") {
          filterKind = "pinned";
        } else if (kind === "code" || kind === "url" || kind === "prompt" || kind === "text") {
          filterKind = "type";
          filterValue = kind;
        } else if (typeof kind === "object" && "tag" in kind) {
          filterKind = "tag";
          filterValue = kind.tag;
        }

        const results = await api.listSnippets({
          query: baseQuery,
          filterKind,
          filterValue,
        });
        setItems(results.map(snippetToClipItem));
      } catch (e) {
        console.error("Failed to load snippets:", e);
        setItems([]);
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  const refresh = useCallback(
    (kind: FilterKind = "all") => load(queryRef.current, kind),
    [load],
  );

  useEffect(() => {
    void load("");
  }, [load]);

  const setQuery = (q: string) => {
    queryRef.current = q;
  };

  return { items, loading, load, refresh, setQuery };
}
