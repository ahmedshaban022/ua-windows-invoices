/** A tiny toast store: no external state library needed for four messages. */
import { useEffect, useState } from "react";

export type ToastKind = "info" | "success" | "error";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

let nextId = 1;
let toasts: Toast[] = [];
const listeners = new Set<(items: Toast[]) => void>();

function emit() {
  for (const listener of listeners) listener(toasts);
}

/** Show a toast message. */
export function toast(message: string, kind: ToastKind = "info") {
  const item: Toast = { id: nextId++, kind, message };
  toasts = [...toasts, item];
  emit();
  window.setTimeout(() => dismissToast(item.id), kind === "error" ? 7000 : 3500);
}

/** Remove a toast. */
export function dismissToast(id: number) {
  toasts = toasts.filter((item) => item.id !== id);
  emit();
}

/** Subscribe a component to the toast list. */
export function useToasts(): Toast[] {
  const [items, setItems] = useState<Toast[]>(toasts);
  useEffect(() => {
    listeners.add(setItems);
    setItems(toasts);
    return () => {
      listeners.delete(setItems);
    };
  }, []);
  return items;
}
