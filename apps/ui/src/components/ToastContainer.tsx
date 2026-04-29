import { For } from "solid-js";
import { getToasts, dismissToast } from "../stores/toastStore";
import type { Toast } from "../stores/toastStore";

export function ToastContainer() {
  const toasts = getToasts;

  const getIcon = (type: Toast["type"]) => {
    if (type === "success") return "✓";
    if (type === "error") return "✕";
    if (type === "warning") return "⚠";
    if (type === "info") return "ℹ";
    return "•";
  };

  const getBg = (type: Toast["type"]) => {
    if (type === "success") return "#d1fae5";
    if (type === "error") return "#fee2e2";
    if (type === "warning") return "#fef3c7";
    if (type === "info") return "#dbeafe";
    return "#f3f4f6";
  };

  const getTextColor = (type: Toast["type"]) => {
    if (type === "success") return "#065f46";
    if (type === "error") return "#991b1b";
    if (type === "warning") return "#b45309";
    if (type === "info") return "#0c4a6e";
    return "#374151";
  };

  const getIconColor = (type: Toast["type"]) => {
    if (type === "success") return "#10b981";
    if (type === "error") return "#ef4444";
    if (type === "warning") return "#f59e0b";
    if (type === "info") return "#3b82f6";
    return "#6b7280";
  };

  return (
    <div
      style={{
        "position": "fixed",
        "top": "16px",
        "right": "16px",
        "z-index": "9999",
        "max-width": "400px",
      }}
    >
      <For each={toasts()}>
        {(toast) => (
          <div
            style={{
              "margin-bottom": "8px",
              "background-color": getBg(toast.type),
              "border-left": `4px solid ${getIconColor(toast.type)}`,
              "border-radius": "4px",
              padding: "12px 16px",
              "color": getTextColor(toast.type),
              "font-size": "14px",
              "box-shadow": "0 2px 8px rgba(0,0,0,0.1)",
              display: "flex",
              gap: "12px",
              "align-items": "flex-start",
              animation: "slideIn 0.3s ease-out",
            }}
          >
            <div
              style={{
                "font-weight": "bold",
                "font-size": "18px",
                "flex-shrink": 0,
                "color": getIconColor(toast.type),
                "line-height": "1",
              }}
            >
              {getIcon(toast.type)}
            </div>

            <div style={{ "flex": "1" }}>
              {toast.message}
            </div>

            <button
              onClick={() => dismissToast(toast.id)}
              style={{
                "background": "none",
                "border": "none",
                "padding": "0",
                "margin": "0",
                "color": getTextColor(toast.type),
                cursor: "pointer",
                opacity: 0.6,
                "font-size": "18px",
                "line-height": "1",
                "flex-shrink": 0,
              }}
              aria-label="Dismiss"
            >
              ✕
            </button>
          </div>
        )}
      </For>

      <style>
        {`
          @keyframes slideIn {
            from {
              transform: translateX(400px);
              opacity: 0;
            }
            to {
              transform: translateX(0);
              opacity: 1;
            }
          }
        `}
      </style>
    </div>
  );
}
