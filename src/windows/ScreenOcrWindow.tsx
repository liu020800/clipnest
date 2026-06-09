import { useEffect, useMemo, useState } from "react";
import { Loader2, ScanText, X } from "lucide-react";
import { api } from "../lib/api";

interface Point {
  x: number;
  y: number;
}

function rectFromPoints(a: Point, b: Point) {
  const left = Math.min(a.x, b.x);
  const top = Math.min(a.y, b.y);
  const width = Math.abs(a.x - b.x);
  const height = Math.abs(a.y - b.y);
  return { left, top, width, height };
}

export function ScreenOcrWindow({
  onToast,
}: {
  onToast: (title: string, description?: string, tone?: "success" | "info" | "warning" | "error") => void;
}) {
  const [start, setStart] = useState<Point | null>(null);
  const [current, setCurrent] = useState<Point | null>(null);
  const [running, setRunning] = useState(false);
  const rect = useMemo(() => start && current ? rectFromPoints(start, current) : null, [start, current]);

  useEffect(() => {
    document.body.classList.add("screen-ocr-body");
    setStart(null);
    setCurrent(null);
    setRunning(false);
    return () => document.body.classList.remove("screen-ocr-body");
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void api.hideWindow("screen_ocr");
      }
    };
    document.addEventListener("keydown", onKeyDown, true);
    return () => document.removeEventListener("keydown", onKeyDown, true);
  }, []);

  const finishSelection = async () => {
    if (!rect || running) return;
    if (rect.width < 8 || rect.height < 8) {
      setStart(null);
      setCurrent(null);
      return;
    }

    setRunning(true);
    try {
      const result = await api.captureScreenOcrRegion({
        x: Math.round(window.screenX + rect.left),
        y: Math.round(window.screenY + rect.top),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
      });
      if (!result.text.trim()) {
        onToast("未识别到文字", "可以重新框选更清晰的区域", "info");
        return;
      }
      await api.setPendingCaptureText(result.text);
      await api.openCapture();
      onToast("框选识别完成", `${result.text.length} 字`, "success");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      onToast("框选识别失败", msg, "error");
      await api.hideWindow("screen_ocr").catch(() => {});
    } finally {
      setRunning(false);
      setStart(null);
      setCurrent(null);
    }
  };

  return (
    <div
      className="screen-ocr-root"
      onMouseDown={(event) => {
        if (running) return;
        setStart({ x: event.clientX, y: event.clientY });
        setCurrent({ x: event.clientX, y: event.clientY });
      }}
      onMouseMove={(event) => {
        if (!start || running) return;
        setCurrent({ x: event.clientX, y: event.clientY });
      }}
      onMouseUp={() => void finishSelection()}
    >
      <div className="screen-ocr-toolbar">
        {running ? <Loader2 className="h-4 w-4 animate-spin" /> : <ScanText className="h-4 w-4" />}
        <span>{running ? "正在识别" : "拖拽框选要识别并保存的文字"}</span>
        <button type="button" onClick={() => void api.hideWindow("screen_ocr")} title="取消">
          <X className="h-4 w-4" />
        </button>
      </div>
      {rect && (
        <div
          className="screen-ocr-selection"
          style={{
            left: `${rect.left}px`,
            top: `${rect.top}px`,
            width: `${rect.width}px`,
            height: `${rect.height}px`,
          }}
        />
      )}
      {running && (
        <div className="screen-ocr-busy">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span>正在截取并识别</span>
        </div>
      )}
    </div>
  );
}

export default ScreenOcrWindow;
