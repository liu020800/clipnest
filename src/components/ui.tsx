// 通用 UI 组件
import { type ReactNode, type HTMLAttributes } from "react";
import { Check } from "lucide-react";
import { cn } from "../lib/utils";
import type { GlassVariant, ToastTone } from "../types";

export function AmbientBackground() {
  return <div className="ambient-background" aria-hidden="true" />;
}

export function GlassPanel({
  children,
  className = "",
  variant = "card",
  ...props
}: {
  children: ReactNode;
  className?: string;
  variant?: GlassVariant;
} & HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn("glass-panel", `glass-panel-${variant}`, className)} {...props}>
      {children}
    </div>
  );
}

export function KeyboardChip({ children }: { children: ReactNode }) {
  return <span className="keyboard-chip">{children}</span>;
}

export function SectionHeader({
  title,
  subtitle,
  action,
  icon,
}: {
  title: string;
  subtitle?: ReactNode;
  action?: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <div className="section-header">
      <div>
        <div className="section-header-title">
          {icon}
          {title}
        </div>
        {subtitle && <div className="section-header-subtitle">{subtitle}</div>}
      </div>
      {action}
    </div>
  );
}

export function TagBadge({
  children,
  selected = false,
  ai = false,
  onClick,
}: {
  children: ReactNode;
  selected?: boolean;
  ai?: boolean;
  onClick?: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn("tag-badge", ai && "tag-badge-ai", selected && "tag-badge-selected")}
    >
      {children}
    </button>
  );
}

export function Toast({
  show,
  title,
  description,
  tone = "success",
  onClick,
}: {
  show: boolean;
  title: string;
  description?: string;
  tone?: ToastTone;
  onClick?: () => void;
}) {
  return (
    <button type="button" onClick={onClick} className={cn("toast", `toast-${tone}`, show && "toast-visible")}>
      <div className="toast-icon">
        <Check className="h-4 w-4" />
      </div>
      <div className="text-left">
        <div className="toast-title">{title}</div>
        {description && <div className="toast-description">{description}</div>}
      </div>
    </button>
  );
}

export function Toggle({
  on,
  onChange,
  ariaLabel,
}: {
  on: boolean;
  onChange: (next: boolean) => void;
  ariaLabel?: string;
}) {
  return (
    <button
      type="button"
      className={cn("toggle", on && "toggle-on")}
      onClick={() => onChange(!on)}
      aria-pressed={on}
      aria-label={ariaLabel}
    >
      <span className="toggle-knob" />
    </button>
  );
}
