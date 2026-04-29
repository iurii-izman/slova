import { createEffect, createSignal, Show, For } from "solid-js";
import { useParams, useNavigate } from "@solidjs/router";
import { getJob } from "../stores/queueStore";
import { toastError, toastSuccess, toastWarning } from "../stores/toastStore";
import * as commands from "../ipc/commands";
import type { JobId, Transcript, TranscriptSegment, AppErrorView } from "../ipc/types";
import { formatBytes, formatDuration } from "../utils/formatters";
import { debounceAsync } from "../utils/debounce";

export function DetailPage() {
  const params = useParams();
  const navigate = useNavigate();

  const [transcript, setTranscript] = createSignal<Transcript | null>(null);
  const [segments, setSegments] = createSignal<TranscriptSegment[]>([]);
  const [isEditing, setIsEditing] = createSignal(false);
  const [editText, setEditText] = createSignal("");
  const [isSaving, setIsSaving] = createSignal(false);
  const [isExporting, setIsExporting] = createSignal(false);
  const [autoSaveStatus, setAutoSaveStatus] = createSignal<"idle" | "saving" | "saved" | "error">(
    "idle",
  );
  const [hasUnsavedChanges, setHasUnsavedChanges] = createSignal(false);

  const jobId = params.id as JobId;
  const job = getJob(jobId);

  // Create debounced save function
  const debouncedAutoSave = debounceAsync(
    async (text: string) => {
      try {
        setAutoSaveStatus("saving");
        await commands.saveTranscriptEdit(jobId, text);
        setAutoSaveStatus("saved");
        setHasUnsavedChanges(false);

        // Reset status after 2 seconds
        setTimeout(() => setAutoSaveStatus("idle"), 2000);
      } catch (err) {
        console.error("Auto-save failed:", err);

        // Check if auth failed
        if (err instanceof Error && err.message.includes("AUTH_FAILED")) {
          setAutoSaveStatus("error");
          toastWarning("API key missing or invalid. Go to Settings to configure.");
          setTimeout(() => navigate("/settings"), 2000);
        } else {
          setAutoSaveStatus("error");
          const msg = err instanceof Error ? err.message : String(err);
          toastError(`Auto-save failed: ${msg}`);
          setTimeout(() => setAutoSaveStatus("idle"), 2000);
        }
      }
    },
    1500, // Wait 1.5 seconds before saving
  );

  createEffect(async () => {
    if (!jobId || !job) {
      return;
    }

    try {
      const trans = await commands.getTranscript(jobId);
      setTranscript(trans);
      setEditText(trans.text);

      // Try to load segments from a segments file (if available)
      // For now, we'll just set an empty array
      setSegments([]);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);

      // Check if auth failed
      if (msg.includes("AUTH_FAILED") || msg.includes("api")) {
        toastWarning("API authentication failed. Please check your API key in Settings.");
        setTimeout(() => navigate("/settings"), 2000);
      } else {
        toastError(`Failed to load transcript: ${msg}`);
      }
    }
  });

  // Subscribe to auth-failed event
  createEffect(() => {
    const unsubscribe = commands.onAuthFailed(() => {
      toastWarning("API authentication failed. Please configure your API key in Settings.");
      setTimeout(() => navigate("/settings"), 2000);
    });

    return () => {
      unsubscribe.then((fn) => fn());
    };
  });

  const handleSaveTranscript = async () => {
    try {
      setIsSaving(true);
      const text = editText();
      if (!text.trim()) {
        toastError("Transcript cannot be empty");
        return;
      }

      await commands.saveTranscriptEdit(jobId, text);
      setTranscript({ job_id: jobId, text });
      setIsEditing(false);
      setHasUnsavedChanges(false);
      toastSuccess("Transcript saved");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.includes("AUTH_FAILED")) {
        toastError("API key is invalid. Please check Settings.");
        navigate("/settings");
      } else {
        toastError(`Failed to save: ${msg}`);
      }
    } finally {
      setIsSaving(false);
    }
  };

  const handleExport = async (format: "txt" | "srt" | "json") => {
    try {
      setIsExporting(true);
      const outputPath = await commands.exportJob(jobId, format);
      toastSuccess(`Exported to: ${outputPath}`);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toastError(`Export failed: ${msg}`);
    } finally {
      setIsExporting(false);
    }
  };

  const handleCopyToClipboard = () => {
    const text = transcript()?.text;
    if (!text) return;

    navigator.clipboard.writeText(text).then(() => {
      toastSuccess("Copied to clipboard");
    });
  };

  const handleEditTextChange = (newText: string) => {
    setEditText(newText);
    setHasUnsavedChanges(true);
    debouncedAutoSave(newText);
  };

  const formatSegmentTime = (ms: number): string => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);

    const h = hours.toString().padStart(2, "0");
    const m = (minutes % 60).toString().padStart(2, "0");
    const s = (seconds % 60).toString().padStart(2, "0");

    return `${h}:${m}:${s}`;
  };

  return (
    <div style={{ padding: "24px", "max-width": "1200px", margin: "0 auto" }}>
      {/* Header */}
      <div style={{ "margin-bottom": "24px" }}>
        <button
          onClick={() => navigate("/")}
          style={{
            background: "none",
            border: "none",
            padding: "0",
            color: "#4f46e5",
            cursor: "pointer",
            "font-size": "14px",
            "margin-bottom": "16px",
          }}
        >
          ← Back to Queue
        </button>

        <Show when={job}>
          {(j) => (
            <div
              style={{
                display: "flex",
                "justify-content": "space-between",
                "align-items": "flex-start",
              }}
            >
              <div>
                <h1 style={{ margin: "0 0 8px 0" }}>{j().display_name}</h1>
                <p style={{ margin: "0", color: "#666", "font-size": "14px" }}>
                  {formatBytes(j().size_bytes)} • Status: {j().state.kind}
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
          )}
        </Show>

        <Show when={!job}>
          <div>
            <h1 style={{ margin: "0" }}>Job not found</h1>
          </div>
        </Show>
      </div>

      {/* Show only if job is done */}
      <Show
        when={job && job.state.kind === "Done"}
        fallback={
          <div
            style={{
              "text-align": "center",
              padding: "32px",
              color: "#666",
            }}
          >
            <p>Job is {job ? `in state: ${job.state.kind}` : "loading..."}.</p>
            <p style={{ "font-size": "13px" }}>
              Transcripts are available after processing completes.
            </p>
          </div>
        }
      >
        {/* Toolbar */}
        <div
          style={{
            display: "flex",
            gap: "8px",
            "margin-bottom": "24px",
            "flex-wrap": "wrap",
            "align-items": "center",
          }}
        >
          <button
            onClick={() => setIsEditing(!isEditing())}
            disabled={isEditing() && hasUnsavedChanges()}
            style={{
              padding: "8px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": isEditing() ? "#4f46e5" : "#f3f4f6",
              color: isEditing() ? "white" : "#000",
              cursor: "pointer",
            }}
          >
            {isEditing() ? "Done Editing" : "Edit"}
          </button>

          <button
            onClick={handleCopyToClipboard}
            disabled={!transcript()}
            style={{
              padding: "8px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
            }}
          >
            📋 Copy
          </button>

          <div style={{ flex: "1" }} />

          <Show when={autoSaveStatus() !== "idle"}>
            <span
              style={{
                "font-size": "12px",
                color:
                  autoSaveStatus() === "saving"
                    ? "#f59e0b"
                    : autoSaveStatus() === "saved"
                      ? "#10b981"
                      : "#ef4444",
              }}
            >
              {autoSaveStatus() === "saving" && "💾 Saving..."}
              {autoSaveStatus() === "saved" && "✓ Saved"}
              {autoSaveStatus() === "error" && "✗ Save failed"}
            </span>
          </Show>

          <button
            onClick={() => handleExport("txt")}
            disabled={isExporting()}
            style={{
              padding: "8px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
            }}
          >
            TXT
          </button>

          <button
            onClick={() => handleExport("srt")}
            disabled={isExporting()}
            style={{
              padding: "8px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
            }}
          >
            SRT
          </button>

          <button
            onClick={() => handleExport("json")}
            disabled={isExporting()}
            style={{
              padding: "8px 12px",
              "font-size": "13px",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              "background-color": "#f3f4f6",
              cursor: "pointer",
            }}
          >
            JSON
          </button>
        </div>

        {/* Main content area */}
        <div style={{ display: "grid", "grid-template-columns": "1fr", gap: "24px" }}>
          {/* Transcript Editor or Viewer */}
          <Show when={transcript()}>
            {(trans) => (
              <div>
                <Show
                  when={isEditing()}
                  fallback={
                    <div
                      style={{
                        "background-color": "#fff",
                        border: "1px solid #e5e7eb",
                        "border-radius": "8px",
                        padding: "16px",
                        "white-space": "pre-wrap",
                        "word-wrap": "break-word",
                        "line-height": "1.6",
                        "font-family": "monospace",
                        "font-size": "14px",
                        "max-height": "600px",
                        overflow: "auto",
                      }}
                    >
                      {trans().text || "(empty)"}
                    </div>
                  }
                >
                  <div>
                    <div
                      style={{
                        display: "flex",
                        "justify-content": "space-between",
                        "align-items": "center",
                        "margin-bottom": "8px",
                      }}
                    >
                      <label style={{ "font-weight": "500", "font-size": "13px" }}>
                        Edit Transcript
                      </label>
                      <span
                        style={{
                          "font-size": "11px",
                          color: hasUnsavedChanges() ? "#f59e0b" : "#666",
                        }}
                      >
                        {hasUnsavedChanges() ? "Unsaved changes" : "All changes saved"}
                      </span>
                    </div>
                    <textarea
                      value={editText()}
                      onInput={(e) => handleEditTextChange(e.currentTarget.value)}
                      style={{
                        width: "100%",
                        "min-height": "400px",
                        border: "1px solid #4f46e5",
                        "border-radius": "8px",
                        padding: "12px",
                        "font-family": "monospace",
                        "font-size": "14px",
                        "line-height": "1.6",
                        "box-sizing": "border-box",
                      }}
                    />

                    <div style={{ "margin-top": "12px", display: "flex", gap: "8px" }}>
                      <button
                        onClick={handleSaveTranscript}
                        disabled={isSaving() || !hasUnsavedChanges()}
                        style={{
                          padding: "8px 16px",
                          "font-size": "13px",
                          "background-color": "#10b981",
                          color: "white",
                          border: "none",
                          "border-radius": "4px",
                          cursor: "pointer",
                        }}
                      >
                        {isSaving() ? "Saving..." : "Save Changes"}
                      </button>

                      <button
                        onClick={() => {
                          setEditText(trans().text);
                          setIsEditing(false);
                          setHasUnsavedChanges(false);
                        }}
                        style={{
                          padding: "8px 16px",
                          "font-size": "13px",
                          "background-color": "#ef4444",
                          color: "white",
                          border: "none",
                          "border-radius": "4px",
                          cursor: "pointer",
                        }}
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                </Show>
              </div>
            )}
          </Show>

          {/* Segments List */}
          <Show when={segments().length > 0}>
            <div
              style={{
                border: "1px solid #e5e7eb",
                "border-radius": "8px",
                overflow: "hidden",
              }}
            >
              <div
                style={{
                  "background-color": "#f9fafb",
                  padding: "12px 16px",
                  "border-bottom": "1px solid #e5e7eb",
                  "font-weight": "500",
                  "font-size": "13px",
                }}
              >
                📍 Segments ({segments().length})
              </div>
              <div
                style={{
                  "max-height": "400px",
                  overflow: "auto",
                }}
              >
                <For each={segments()}>
                  {(segment, index) => (
                    <div
                      style={{
                        padding: "12px 16px",
                        "border-bottom":
                          index() === segments().length - 1 ? "none" : "1px solid #e5e7eb",
                        cursor: "pointer",
                        transition: "background-color 0.2s ease",
                        "background-color": "transparent",
                      }}
                      onMouseEnter={(el) => {
                        const target = el.currentTarget as HTMLDivElement;
                        if (target) target.style.backgroundColor = "#f9fafb";
                      }}
                      onMouseLeave={(el) => {
                        const target = el.currentTarget as HTMLDivElement;
                        if (target) target.style.backgroundColor = "transparent";
                      }}
                    >
                      <div
                        style={{
                          display: "flex",
                          "justify-content": "space-between",
                          "align-items": "start",
                        }}
                      >
                        <div style={{ flex: "1" }}>
                          <p
                            style={{
                              margin: "0 0 4px 0",
                              "font-size": "12px",
                              color: "#4f46e5",
                              "font-weight": "500",
                            }}
                          >
                            {formatSegmentTime(segment.start_ms)} →{" "}
                            {formatSegmentTime(segment.end_ms)}
                          </p>
                          <p
                            style={{
                              margin: "0",
                              "font-size": "13px",
                              "line-height": "1.5",
                              color: "#374151",
                            }}
                          >
                            {segment.text}
                          </p>
                        </div>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}
