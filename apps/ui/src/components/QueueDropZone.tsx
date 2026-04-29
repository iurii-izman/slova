import { createSignal } from "solid-js";
import { addFilesToQueue } from "../stores/queueStore";
import { toastError, toastSuccess, toastWarning } from "../stores/toastStore";
import { selectVideoFiles } from "../utils/dialog";

type Props = {
  onFilesAdded?: () => void;
};

export function QueueDropZone(props: Props) {
  const [isDragging, setIsDragging] = createSignal(false);
  const [isProcessing, setIsProcessing] = createSignal(false);

  const handleDragOver = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(true);
  };

  const handleDragLeave = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);
  };

  const handleDrop = async (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setIsDragging(false);

    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) {
      toastWarning("No files found in drop");
      return;
    }

    // For now, we can't directly access file paths from drop event in web context
    // Show a message directing user to use the file picker button
    toastInfo(
      `Detected ${files.length} file(s). Please use "Select Files" button to add them properly.`,
    );
  };

  const handleSelectFiles = async () => {
    try {
      setIsProcessing(true);

      const filePaths = await selectVideoFiles();
      if (filePaths.length === 0) {
        return; // User cancelled
      }

      // Add files to queue
      const jobIds = await addFilesToQueue(filePaths);
      toastSuccess(`Added ${jobIds.length} file(s) to queue`);
      props.onFilesAdded?.();
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toastError(`Failed to add files: ${msg}`);
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <div
      class={`queue-drop-zone ${isDragging() ? "dragging" : ""}`}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      style={{
        border: `2px dashed ${isDragging() ? "#4f46e5" : "#ccc"}`,
        "border-radius": "8px",
        padding: "32px",
        "text-align": "center",
        "background-color": isDragging() ? "#eef2ff" : "#f9fafb",
        transition: "all 0.2s ease",
        "min-height": "200px",
        display: "flex",
        "flex-direction": "column",
        "align-items": "center",
        "justify-content": "center",
        gap: "16px",
        cursor: isDragging() ? "copy" : "default",
      }}
    >
      <div style={{ "font-size": "32px" }}>{isDragging() ? "📁" : "📹"}</div>

      <div>
        <h3 style={{ margin: "0 0 8px 0" }}>
          {isDragging() ? "Drop files here" : "Drag and drop video files"}
        </h3>
        <p style={{ margin: "0", color: "#666", "font-size": "14px" }}>
          Supported formats: MP4, MKV, WebM
        </p>
      </div>

      <div style={{ "font-size": "14px", color: "#666" }}>or</div>

      <button
        disabled={isProcessing()}
        onClick={handleSelectFiles}
        style={{
          "background-color": "#4f46e5",
          color: "white",
          border: "none",
          padding: "8px 16px",
          "border-radius": "4px",
          cursor: isProcessing() ? "not-allowed" : "pointer",
          opacity: isProcessing() ? 0.6 : 1,
        }}
      >
        {isProcessing() ? "Adding..." : "Select Files"}
      </button>
    </div>
  );
}

// Import toastInfo here (was missing before)
import { toastInfo } from "../stores/toastStore";
