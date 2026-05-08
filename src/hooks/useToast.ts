import { useState, useCallback } from "react";
import type { PREvent } from "../types";
import { generateId } from "../types";

export interface Toast extends PREvent {
  id: string;
}

const TOAST_DURATION_MS = 5000;

export function useToast() {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const addToast = useCallback((event: PREvent) => {
    const id = generateId();
    setToasts((prev) => [...prev, { ...event, id }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, TOAST_DURATION_MS);
  }, []);

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return { toasts, addToast, dismissToast };
}
