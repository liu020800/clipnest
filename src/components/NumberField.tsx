import { useState, useEffect } from "react";
import { cn } from "../lib/utils";

export function NumberField({
  value,
  onCommit,
  min,
  max,
  widthClass = "w-24",
}: {
  value: number;
  onCommit: (next: number) => void;
  min?: number;
  max?: number;
  widthClass?: string;
}) {
  const [local, setLocal] = useState(String(value));
  useEffect(() => {
    setLocal(String(value));
  }, [value]);
  const commit = () => {
    const n = Number(local);
    if (!Number.isFinite(n)) {
      setLocal(String(value));
      return;
    }
    if (min !== undefined && n < min) {
      setLocal(String(value));
      return;
    }
    if (max !== undefined && n > max) {
      setLocal(String(value));
      return;
    }
    if (n !== value) onCommit(n);
    else setLocal(String(n));
  };
  return (
    <input
      type="number"
      className={cn("settings-text-input", widthClass)}
      value={local}
      min={min}
      max={max}
      onChange={(e) => setLocal(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        if (e.key === "Escape") {
          setLocal(String(value));
          (e.target as HTMLInputElement).blur();
        }
      }}
    />
  );
}
