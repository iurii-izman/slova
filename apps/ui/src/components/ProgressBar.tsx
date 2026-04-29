import { createMemo } from "solid-js";
import type { JobState } from "../ipc/types";

type Props = {
  state: JobState;
};

export function ProgressBar(props: Props) {
  const progressPercent = createMemo(() => {
    const state = props.state;

    if (state.kind === "Extracting") {
      return Math.round(state.data.progress * 100);
    }
    if (state.kind === "Uploading") {
      return Math.round(state.data.progress * 100);
    }
    if (state.kind === "Chunking") {
      return Math.round(state.data.progress * 100);
    }
    if (state.kind === "Done") {
      return 100;
    }
    if (state.kind === "Failed") {
      return 0;
    }

    return undefined; // indeterminate
  });

  const isIndeterminate = () => progressPercent() === undefined;
  const percent = progressPercent() ?? 0;

  const statusText = createMemo(() => {
    const state = props.state;
    if (state.kind === "Queued") return "Queued";
    if (state.kind === "Probing") return "Checking file...";
    if (state.kind === "Extracting") return "Extracting audio...";
    if (state.kind === "Chunking") return "Chunking audio...";
    if (state.kind === "Uploading") return "Uploading...";
    if (state.kind === "Transcribing") {
      return `Transcribing (chunk ${state.data.chunk_idx}/${state.data.chunk_total})...`;
    }
    if (state.kind === "Stitching") return "Stitching chunks...";
    if (state.kind === "Postprocessing") return "Post-processing...";
    if (state.kind === "Done") return "Done";
    if (state.kind === "Failed") return "Failed";
    if (state.kind === "Cancelled") return "Cancelled";
    if (state.kind === "Paused") return "Paused";
    return "Unknown";
  });

  return (
    <div style={{ width: "100%" }}>
      <div
        style={{
          display: "flex",
          "justify-content": "space-between",
          "align-items": "center",
          "margin-bottom": "8px",
        }}
      >
        <span style={{ "font-size": "14px", color: "#666" }}>
          {statusText()}
        </span>
        {!isIndeterminate() && (
          <span style={{ "font-size": "13px", "font-weight": "bold" }}>
            {percent}%
          </span>
        )}
      </div>

      <div
        style={{
          width: "100%",
          height: "6px",
          "background-color": "#e5e7eb",
          "border-radius": "3px",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            height: "100%",
            width: isIndeterminate() ? "100%" : `${percent}%`,
            "background-color": getBarColor(props.state),
            transition: "width 0.3s ease",
            ...(isIndeterminate()
              ? {
                  animation: "pulse 1.5s ease-in-out infinite",
                  opacity: 0.7,
                }
              : {}),
          }}
        />
      </div>

      <style>
        {`
          @keyframes pulse {
            0%, 100% { opacity: 0.7; }
            50% { opacity: 0.3; }
          }
        `}
      </style>
    </div>
  );
}

function getBarColor(state: JobState): string {
  if (state.kind === "Done") return "#10b981"; // green
  if (state.kind === "Failed") return "#ef4444"; // red
  if (state.kind === "Cancelled") return "#f97316"; // orange
  if (state.kind === "Paused") return "#eab308"; // yellow
  return "#4f46e5"; // blue (default)
}
