import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { Loader2, Maximize2 } from "lucide-react";
import { cn } from "../lib/utils";

export function ImagePreview({
  relPath,
  alt = "预览",
  className,
}: {
  relPath: string;
  alt?: string;
  className?: string;
}) {
  const [src, setSrc] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [enlarged, setEnlarged] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setSrc(null);
    setError(null);
    api
      .resolveImagePath(relPath)
      .then((abs) => api.toAssetUrl(abs))
      .then((url) => {
        if (!cancelled) setSrc(url);
      })
      .catch((e) => {
        if (!cancelled) setError(typeof e === "string" ? e : String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [relPath]);

  if (error) {
    return (
      <div className={cn("image-preview image-preview-error", className)}>
        图片加载失败: {error}
      </div>
    );
  }
  if (!src) {
    return (
      <div className={cn("image-preview image-preview-loading", className)}>
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>加载图片…</span>
      </div>
    );
  }
  return (
    <>
      <div className={cn("image-preview", className)}>
        <img src={src} alt={alt} loading="lazy" />
        <button
          type="button"
          className="image-preview-zoom"
          aria-label="放大查看"
          onClick={() => setEnlarged(true)}
        >
          <Maximize2 className="h-3.5 w-3.5" />
        </button>
      </div>
      {enlarged && (
        <div
          role="dialog"
          aria-label="原图"
          className="image-zoom-backdrop"
          onClick={() => setEnlarged(false)}
        >
          <img src={src} alt={alt} className="image-zoom-img" />
        </div>
      )}
    </>
  );
}
