import { createEffect, For, Show, createSignal } from "solid-js";
import { useNavigate } from "@solidjs/router";
import {
  initQueueStore,
  destroyQueueStore,
  getFilteredJobs,
  getFilter,
  setFilter,
  cancelJob,
  retryJob,
  getJobs,
  getError,
  isLoading,
} from "../stores/queueStore";
import { QueueDropZone } from "../components/QueueDropZone";
import { JobCard } from "../components/JobCard";
import { toastError, toastSuccess } from "../stores/toastStore";
import type { JobId } from "../ipc/types";

export function QueuePage() {
  const navigate = useNavigate();
  const [isPausedUI, setIsPausedUI] = createSignal(false);

  // Initialize store on mount
  createEffect(async () => {
    try {
      await initQueueStore();
    } catch (err) {
      console.error("Failed to initialize queue:", err);
      toastError("Failed to load queue");
    }

    // Cleanup on unmount
    return async () => {
      await destroyQueueStore();
    };
  });

  const handleCancelJob = async (id: JobId) => {
    try {
      await cancelJob(id);
      toastSuccess("Job cancelled");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toastError(`Failed to cancel: ${msg}`);
    }
  };

  const handleRetryJob = async (id: JobId) => {
    try {
      await retryJob(id);
      toastSuccess("Job retry started");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toastError(`Failed to retry: ${msg}`);
    }
  };

  const jobCounts = {
    all: () => getJobs().length,
    active: () =>
      getJobs().filter(
        (j) => j.state.kind !== "Done" && j.state.kind !== "Failed" && j.state.kind !== "Cancelled",
      ).length,
    failed: () => getJobs().filter((j) => j.state.kind === "Failed").length,
    done: () => getJobs().filter((j) => j.state.kind === "Done").length,
  };

  return (
    <div style={{ padding: "24px", "max-width": "900px", margin: "0 auto" }}>
      {/* Header */}
      <div
        style={{
          "margin-bottom": "32px",
          display: "flex",
          "justify-content": "space-between",
          "align-items": "flex-start",
        }}
      >
        <div>
          <h1 style={{ margin: "0 0 8px 0" }}>Slova</h1>
          <p style={{ margin: "0", color: "#666", "font-size": "14px" }}>
            Fast batch transcription with Groq Whisper API
          </p>
        </div>
        <button
          onClick={() => navigate("/settings")}
          style={{
            padding: "6px 12px",
            "font-size": "13px",
            border: "1px solid #d1d5db",
            "border-radius": "4px",
            "background-color": "#f3f4f6",
            cursor: "pointer",
          }}
        >
          ⚙ Settings
        </button>
      </div>

      {/* Global error message */}
      <Show when={getError()}>
        <div
          style={{
            "background-color": "#fee2e2",
            border: "1px solid #fecaca",
            "border-radius": "4px",
            padding: "12px",
            color: "#991b1b",
            "margin-bottom": "16px",
            "font-size": "14px",
          }}
        >
          ⚠ {getError()}
        </div>
      </Show>

      {/* Drop Zone */}
      <div style={{ "margin-bottom": "32px" }}>
        <QueueDropZone />
      </div>

      {/* Controls Bar */}
      <div
        style={{
          display: "flex",
          "justify-content": "space-between",
          "align-items": "center",
          "margin-bottom": "16px",
        }}
      >
        <div>
          <h2 style={{ margin: "0", "font-size": "18px" }}>Queue</h2>
        </div>

        <div style={{ display: "flex", gap: "8px" }}>
          <button
            disabled={isPausedUI() || getJobs().length === 0}
            onClick={() => setIsPausedUI(!isPausedUI())}
            style={{
              padding: "6px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
            }}
          >
            {isPausedUI() ? "Resume" : "Pause"}
          </button>
        </div>
      </div>

      {/* Filters */}
      <div
        style={{
          display: "flex",
          gap: "12px",
          "margin-bottom": "24px",
          "border-bottom": "1px solid #e5e7eb",
          "padding-bottom": "12px",
        }}
      >
        {(["all", "active", "failed", "done"] as const).map((f) => (
          <button
            onClick={() => setFilter(f)}
            style={{
              padding: "6px 12px",
              "font-size": "13px",
              border: "none",
              background: getFilter() === f ? "#4f46e5" : "transparent",
              color: getFilter() === f ? "white" : "#666",
              cursor: "pointer",
              "border-radius": "4px",
              transition: "all 0.2s ease",
            }}
          >
            {f.charAt(0).toUpperCase() + f.slice(1)} ({jobCounts[f]()})
          </button>
        ))}
      </div>

      {/* Loading state */}
      <Show when={isLoading()}>
        <div style={{ "text-align": "center", color: "#666", padding: "24px" }}>
          Loading queue...
        </div>
      </Show>

      {/* Jobs list */}
      <Show when={!isLoading()}>
        <div>
          <Show
            when={getFilteredJobs().length > 0}
            fallback={
              <div
                style={{
                  "text-align": "center",
                  color: "#999",
                  padding: "32px 0",
                  "font-size": "14px",
                }}
              >
                {getFilter() === "all"
                  ? "No jobs yet. Add some files above!"
                  : `No ${getFilter()} jobs.`}
              </div>
            }
          >
            <div>
              <For each={getFilteredJobs()}>
                {(job) => <JobCard job={job} onCancel={handleCancelJob} onRetry={handleRetryJob} />}
              </For>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}
