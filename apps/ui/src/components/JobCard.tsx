import { createMemo, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import type { Job, JobId } from "../ipc/types";
import { ProgressBar } from "./ProgressBar";
import { formatBytes, formatDate } from "../utils/formatters";

type Props = {
  job: Job;
  onCancel?: (id: JobId) => void;
  onRetry?: (id: JobId) => void;
  onExport?: (id: JobId) => void;
};

export function JobCard(props: Props) {
  const navigate = useNavigate();

  const canRetry = createMemo(
    () => props.job.state.kind === "Failed" || props.job.state.kind === "Cancelled",
  );

  const canCancel = createMemo(
    () =>
      props.job.state.kind !== "Done" &&
      props.job.state.kind !== "Failed" &&
      props.job.state.kind !== "Cancelled",
  );

  const canExport = createMemo(() => props.job.state.kind === "Done");

  const displayDate = createMemo(() => {
    try {
      const date = new Date(props.job.created_at);
      return formatDate(date);
    } catch {
      return props.job.created_at;
    }
  });

  const errorMessage = createMemo(() => {
    const state = props.job.state;
    if (state.kind === "Failed") {
      return state.data.error.message;
    }
    return null;
  });

  return (
    <div
      style={{
        border: "1px solid #e5e7eb",
        "border-radius": "8px",
        padding: "16px",
        "background-color": "#fff",
        "margin-bottom": "12px",
      }}
    >
      {/* Header: Name and Date */}
      <div
        style={{
          display: "flex",
          "justify-content": "space-between",
          "align-items": "flex-start",
          "margin-bottom": "12px",
        }}
      >
        <div>
          <div
            style={{ display: "flex", "align-items": "center", gap: "8px", "margin-bottom": "4px" }}
          >
            <h4 style={{ margin: "0", "font-size": "16px" }}>{props.job.display_name}</h4>
            {props.job.settings_snapshot.enable_postprocess && (
              <span
                title="Postprocessing enabled"
                style={{
                  padding: "2px 6px",
                  "border-radius": "2px",
                  "background-color": "#dbeafe",
                  color: "#1e40af",
                  "font-size": "10px",
                  "font-weight": "bold",
                }}
              >
                Postprocessed
              </span>
            )}
          </div>
          <p style={{ margin: "0", "font-size": "12px", color: "#999" }}>
            {formatBytes(props.job.size_bytes)} • {displayDate()}
          </p>
        </div>

        {/* Status badge */}
        <div
          style={{
            padding: "4px 12px",
            "border-radius": "4px",
            "font-size": "12px",
            "font-weight": "bold",
            "background-color": getStatusBg(props.job.state.kind),
            color: getStatusColor(props.job.state.kind),
          }}
        >
          {getStatusLabel(props.job.state.kind)}
        </div>
      </div>

      {/* Progress bar */}
      <div style={{ "margin-bottom": "12px" }}>
        <ProgressBar state={props.job.state} />
      </div>

      {/* Error message */}
      <Show when={errorMessage()}>
        <div
          style={{
            "background-color": "#fee2e2",
            border: "1px solid #fecaca",
            "border-radius": "4px",
            padding: "8px 12px",
            color: "#991b1b",
            "font-size": "13px",
            "margin-bottom": "12px",
          }}
        >
          {errorMessage()}
        </div>
      </Show>

      {/* Action buttons */}
      <div
        style={{
          display: "flex",
          gap: "8px",
        }}
      >
        <Show when={canCancel()}>
          <button
            onClick={() => props.onCancel?.(props.job.id)}
            style={{
              padding: "6px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
              "white-space": "nowrap",
            }}
          >
            Cancel
          </button>
        </Show>

        <Show when={canRetry()}>
          <button
            onClick={() => props.onRetry?.(props.job.id)}
            style={{
              padding: "6px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
              "white-space": "nowrap",
            }}
          >
            Retry
          </button>
        </Show>

        <Show when={canExport()}>
          <button
            onClick={() => navigate(`/detail/${props.job.id}`)}
            style={{
              padding: "6px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
              "white-space": "nowrap",
            }}
          >
            View
          </button>
        </Show>

        <Show when={canExport()}>
          <button
            onClick={() => props.onExport?.(props.job.id)}
            style={{
              padding: "6px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
              "white-space": "nowrap",
            }}
          >
            Export
          </button>
        </Show>
      </div>
    </div>
  );
}

function getStatusLabel(kind: string): string {
  const labels: Record<string, string> = {
    Queued: "Queued",
    Probing: "Checking",
    Extracting: "Extracting",
    Chunking: "Chunking",
    Uploading: "Uploading",
    Transcribing: "Transcribing",
    Stitching: "Stitching",
    Postprocessing: "Postprocessing",
    Done: "Done",
    Failed: "Failed",
    Cancelled: "Cancelled",
    Paused: "Paused",
  };
  return labels[kind] || kind;
}

function getStatusBg(kind: string): string {
  if (kind === "Done") return "#d1fae5"; // light green
  if (kind === "Failed") return "#fee2e2"; // light red
  if (kind === "Cancelled") return "#fed7aa"; // light orange
  if (kind === "Paused") return "#fef3c7"; // light yellow
  return "#ede9fe"; // light blue (processing)
}

function getStatusColor(kind: string): string {
  if (kind === "Done") return "#065f46"; // dark green
  if (kind === "Failed") return "#991b1b"; // dark red
  if (kind === "Cancelled") return "#92400e"; // dark orange
  if (kind === "Paused") return "#b45309"; // dark yellow
  return "#4f46e5"; // blue (processing)
}
