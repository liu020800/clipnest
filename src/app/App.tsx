// 应用根组件: 根据 Tauri window label 分发到不同窗口
import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { CaptureWindow } from "../windows/CaptureWindow";
import { SearchWindow } from "../windows/SearchWindow";
import { SettingsWindow } from "../windows/SettingsWindow";
import { ScreenOcrWindow } from "../windows/ScreenOcrWindow";
import { Toast } from "../components/ui";
import { useToast } from "../hooks/useToast";
import { safeGetCurrentWindowLabel } from "../lib/api";

const WINDOW_LABELS = ["capture", "search", "settings", "screen_ocr"] as const;

function normalizeWindowLabel(raw: string): "capture" | "search" | "settings" | "screen_ocr" {
  if ((WINDOW_LABELS as readonly string[]).includes(raw)) {
    return raw as "capture" | "search" | "settings" | "screen_ocr";
  }
  console.warn(`[App] Unknown window label "${raw}", falling back to search window`);
  return "search";
}

export function App() {
  const [label, setLabel] = useState("");
  const { toast, showToast } = useToast();

  useEffect(() => {
    const url = new URL(window.location.href);
    const override = url.searchParams.get("window");
    if (override && (WINDOW_LABELS as readonly string[]).includes(override)) {
      setLabel(override);
      return;
    }
    setLabel(normalizeWindowLabel(safeGetCurrentWindowLabel()));
  }, []);

  if (!label) {
    return (
      <div className="app-root">
        <div className="flex h-full items-center justify-center">
          <Loader2 className="h-5 w-5 animate-spin text-[var(--accent)]" />
        </div>
      </div>
    );
  }

  return (
    <div className="app-root">
      {label === "capture" && <CaptureWindow onToast={showToast} />}
      {label === "search" && <SearchWindow onToast={showToast} />}
      {label === "settings" && <SettingsWindow onToast={showToast} />}
      {label === "screen_ocr" && <ScreenOcrWindow onToast={showToast} />}
      <Toast
        show={toast.show}
        title={toast.title}
        description={toast.description}
        tone={toast.tone}
      />
    </div>
  );
}

export default App;
