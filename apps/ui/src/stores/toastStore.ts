import { createStore } from "solid-js/store";
import { createEffect } from "solid-js";

export type Toast = {
  id: string;
  type: "success" | "error" | "warning" | "info";
  message: string;
  duration: number | null; // ms, null = no auto-dismiss
};

const [toasts, setToasts] = createStore<Toast[]>([]);

let idCounter = 0;

/**
 * Show a toast notification
 */
export function showToast(
  message: string,
  type: Toast["type"] = "info",
  duration: number = 5000,
): string {
  const id = `toast-${idCounter++}`;

  const newToast: Toast = {
    id,
    type,
    message,
    duration: duration > 0 ? duration : null,
  };
  setToasts((prev) => [...prev, newToast]);

  // Auto-dismiss if duration > 0
  if (duration > 0) {
    setTimeout(() => {
      dismissToast(id);
    }, duration);
  }

  return id;
}

/**
 * Dismiss a toast by id
 */
export function dismissToast(id: string) {
  setToasts((prev) => prev.filter((t) => t.id !== id));
}

/**
 * Get all toasts
 */
export function getToasts(): Toast[] {
  return toasts;
}

/**
 * Convenience helpers
 */
export function toastSuccess(message: string, duration = 5000) {
  return showToast(message, "success", duration);
}

export function toastError(message: string, duration = 7000) {
  return showToast(message, "error", duration);
}

export function toastWarning(message: string, duration = 6000) {
  return showToast(message, "warning", duration);
}

export function toastInfo(message: string, duration = 5000) {
  return showToast(message, "info", duration);
}
