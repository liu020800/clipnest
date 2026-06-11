// 键盘导航 hook: ↑/↓ 切换选中项, Enter 触发 onEnter
import { useEffect } from "react";

interface UseKeyboardNavigationOpts {
  enabled: boolean;
  count: number;
  selectedIndex: number;
  setSelectedIndex: (i: number) => void;
  onEnter: () => void;
  onEscape?: () => void;
  onCopyKeepOpen?: () => void;
}

export function useKeyboardNavigation({
  enabled,
  count,
  selectedIndex,
  setSelectedIndex,
  onEnter,
  onEscape,
  onCopyKeepOpen,
}: UseKeyboardNavigationOpts) {
  useEffect(() => {
    if (!enabled) return;
    const handler = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      const tag = target?.tagName;
      const editingText = tag === "INPUT" || tag === "TEXTAREA";

      if (editingText) {
        if (e.key === "ArrowDown") {
          e.preventDefault();
          if (count === 0) return;
          setSelectedIndex((selectedIndex + 1) % count);
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          if (count === 0) return;
          setSelectedIndex((selectedIndex - 1 + count) % count);
        } else if (e.key === "Enter") {
          e.preventDefault();
          onEnter();
        } else if (e.key === "Escape") {
          e.preventDefault();
          onEscape?.();
        }
        return;
      }

      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (count === 0) return;
        setSelectedIndex((selectedIndex + 1) % count);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        if (count === 0) return;
        setSelectedIndex((selectedIndex - 1 + count) % count);
      } else if (e.key === "Enter") {
        e.preventDefault();
        onEnter();
      } else if (e.key === "Escape") {
        e.preventDefault();
        onEscape?.();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "c") {
        e.preventDefault();
        onCopyKeepOpen?.();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [enabled, count, selectedIndex, setSelectedIndex, onEnter, onEscape, onCopyKeepOpen]);
}
