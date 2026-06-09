// Toast 状态管理
import { useCallback, useRef, useState } from "react";
import type { ToastTone } from "../types";

export interface ToastState {
  title: string;
  description?: string;
  show: boolean;
  tone?: ToastTone;
}

export function useToast() {
  const [toast, setToast] = useState<ToastState>({
    title: "",
    description: "",
    show: false,
  });
  const timerRef = useRef<number | null>(null);

  const showToast = useCallback(
    (title: string, description?: string, tone: ToastTone = "success") => {
      if (timerRef.current) window.clearTimeout(timerRef.current);
      setToast({ title, description, show: true, tone });
      timerRef.current = window.setTimeout(() => {
        setToast((c) => ({ ...c, show: false }));
        timerRef.current = null;
      }, 2000);
    },
    [],
  );

  const dismiss = useCallback(() => {
    if (timerRef.current) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    setToast((c) => ({ ...c, show: false }));
  }, []);

  return { toast, showToast, dismiss };
}
