// 集中管理所有设置
import { useEffect, useState, useCallback } from "react";
import { api } from "../lib/api";

export const SETTINGS_DEFAULTS: Record<string, string> = {
  autostart: "false",
  capture_shortcut: "Ctrl+Shift+S",
  capture_shortcut_alt: "Alt+W",
  search_shortcut: "Alt+Space",
  screen_ocr_shortcut: "Ctrl+Shift+O",
  title_max_length: "10",
  search_limit: "50",
  search_debounce_ms: "150",
  auto_close_on_blur: "true",
  auto_close_delay_ms: "150",
  auto_tag_on_capture: "true",
  capture_text_max_length: "50000",
  markdown_export_pinned_only: "false",
  ai_enabled: "false",
  ollama_endpoint: "http://localhost:11434",
  ollama_model: "qwen3:4b",
  ai_tag_fallback: "rules",
  schema_version: "4",
};

export interface SettingsHook {
  settings: Record<string, string>;
  loaded: boolean;
  get: (key: string, fallback?: string) => string;
  getNum: (key: string, fallback: number) => number;
  getBool: (key: string, fallback: boolean) => boolean;
  refresh: () => Promise<void>;
  saveOne: (key: string, value: string) => Promise<void>;
}

export function useSettings(): SettingsHook {
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [loaded, setLoaded] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const data = await api.getAllSettings();
      setSettings(data);
    } catch (e) {
      console.error("Failed to load settings:", e);
    } finally {
      setLoaded(true);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const get = useCallback(
    (key: string, fallback: string = ""): string => {
      const v = settings[key];
      if (v === undefined || v === "") return fallback || SETTINGS_DEFAULTS[key] || "";
      return v;
    },
    [settings],
  );

  const getNum = useCallback(
    (key: string, fallback: number): number => {
      const v = settings[key];
      if (v === undefined) return fallback;
      const n = Number(v);
      return Number.isFinite(n) ? n : fallback;
    },
    [settings],
  );

  const getBool = useCallback(
    (key: string, fallback: boolean): boolean => {
      const v = settings[key];
      if (v === undefined) return fallback;
      return v === "true";
    },
    [settings],
  );

  const saveOne = useCallback(async (key: string, value: string) => {
    try {
      await api.saveSetting(key, value);
      setSettings((s) => ({ ...s, [key]: value }));
    } catch (e) {
      console.error(`Failed to save ${key}:`, e);
      throw e;
    }
  }, []);

  return { settings, loaded, get, getNum, getBool, refresh, saveOne };
}
