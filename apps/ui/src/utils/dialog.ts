/**
 * Tauri dialog helpers for file/folder selection
 * Tauri v2 uses plugin package: @tauri-apps/plugin-dialog
 */

/**
 * Open file dialog to select video files
 * Returns array of file paths
 */
export async function selectVideoFiles(): Promise<string[]> {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");

    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Video Files",
          extensions: ["mp4", "mkv", "webm"],
        },
        {
          name: "All Files",
          extensions: ["*"],
        },
      ],
    });

    if (!selected) return [];
    if (Array.isArray(selected)) return selected;
    return [selected];
  } catch (err) {
    console.error("Failed to open file dialog:", err);
    throw new Error(`File dialog error: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Open folder dialog to select directory
 * Returns folder path
 */
export async function selectFolder(): Promise<string | null> {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");

    const selected = await open({ directory: true });

    if (!selected || Array.isArray(selected)) return null;
    return selected;
  } catch (err) {
    console.error("Failed to open folder dialog:", err);
    throw new Error(`Folder dialog error: ${err instanceof Error ? err.message : String(err)}`);
  }
}

/**
 * Show a message dialog
 */
export async function showMessage(
  title: string,
  messageText: string,
  type: "info" | "warning" | "error" = "info",
): Promise<void> {
  try {
    const { message } = await import("@tauri-apps/plugin-dialog");
    await message(messageText, { title, kind: type });
  } catch (err) {
    console.error("Failed to show message:", err);
  }
}

/**
 * Show a confirmation dialog
 * Returns true if user clicked OK
 */
export async function showConfirm(title: string, confirmMessage: string): Promise<boolean> {
  try {
    const { ask } = await import("@tauri-apps/plugin-dialog");
    return await ask(confirmMessage, { title, kind: "info" });
  } catch (err) {
    console.error("Failed to show confirm dialog:", err);
    return false;
  }
}
