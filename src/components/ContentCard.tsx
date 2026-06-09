import React, { useEffect, useState } from "react";
import { highlightMatches, cn } from "../lib/utils";
import { api } from "../lib/api";
import { useInViewport } from "../hooks/useInViewport";
import { typeMeta } from "./typeMeta";
import type { ClipItem } from "../types";

function CardThumb({ src, alt }: { src: string; alt: string }) {
  const [resolvedSrc, setResolvedSrc] = useState<string | null>(null);
  const [errored, setErrored] = useState(false);
  useEffect(() => {
    let cancelled = false;
    api.resolveImagePath(src).then((abs) => api.toAssetUrl(abs)).then((url) => {
      if (!cancelled) setResolvedSrc(url);
    }).catch(() => {
      if (!cancelled) setErrored(true);
    });
    return () => { cancelled = true; };
  }, [src]);
  if (errored || !resolvedSrc) return null;
  return <img src={resolvedSrc} alt={alt} loading="lazy" className="content-card-thumb" />;
}

export function ContentCard({
  item,
  selected,
  onClick,
  query,
}: {
  item: ClipItem;
  selected: boolean;
  onClick: () => void;
  query?: string;
}) {
  const meta = typeMeta(item.type);
  const Icon = meta.icon;
  const visibleTags = item.tags.slice(0, 4);
  const hiddenCount = Math.max(item.tags.length - visibleTags.length, 0);
  const hasImage = Boolean(item.image_path);
  const [thumbRef, inView] = useInViewport<HTMLDivElement>("200px");
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn("content-card", selected && "content-card-selected")}
    >
      <div className="flex items-start gap-3">
        <div className="type-icon">
          <Icon className="h-4 w-4" />
        </div>
        <div className="min-w-0 flex-1 text-left">
          <div className="flex items-center justify-between gap-3">
            <h3 className="truncate text-[14px] font-semibold text-[var(--text-main)]">
              {highlightMatches(item.title, query ?? "", React)}
            </h3>
            <span className="shrink-0 text-[11px] text-[var(--text-muted)]">{item.time}</span>
          </div>
          <p className="mt-1.5 line-clamp-2 text-[12.5px] leading-5 text-[var(--text-secondary)]">
            {highlightMatches(item.summary, query ?? "", React)}
          </p>
          {visibleTags.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1.5">
              {visibleTags.map((tag) => (
                <span key={tag} className="mini-tag">
                  {highlightMatches(tag, query ?? "", React)}
                </span>
              ))}
              {hiddenCount > 0 && <span className="mini-tag mini-tag-more">+{hiddenCount}</span>}
            </div>
          )}
        </div>
        {hasImage && (
          <div ref={thumbRef} className="content-card-thumb-frame" data-testid="card-thumb-frame">
            {inView && item.image_path ? <CardThumb src={item.image_path} alt={item.title} /> : null}
          </div>
        )}
      </div>
    </button>
  );
}
