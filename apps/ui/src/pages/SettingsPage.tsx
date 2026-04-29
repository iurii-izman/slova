import { createEffect, createSignal, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import {
  loadSettings,
  saveSettings,
  setApiKey,
  deleteApiKey,
  getSettings,
  getSettingsState,
  clearError,
} from "../stores/settingsStore";
import { toastError, toastSuccess } from "../stores/toastStore";
import type { Settings } from "../ipc/types";

export function SettingsPage() {
  const navigate = useNavigate();

  const [apiKeyInput, setApiKeyInput] = createSignal("");
  const [isShowingApiKey, setIsShowingApiKey] = createSignal(false);
  const [language, setLanguage] = createSignal("ru");
  const [outputFormat, setOutputFormat] = createSignal<"txt" | "srt" | "json">("txt");
  const [parallelism, setParallelism] = createSignal(3);
  const [enablePostprocess, setEnablePostprocess] = createSignal(false);

  // Load settings on mount
  createEffect(async () => {
    await loadSettings();
    const settings = getSettings();
    if (settings) {
      setLanguage(settings.language);
      setOutputFormat(settings.output_format);
      setParallelism(settings.parallelism);
      setEnablePostprocess(settings.enable_postprocess);
    }
  });

  const state = getSettingsState;

  const handleSaveSettings = async () => {
    try {
      const newSettings: Settings = {
        language: language(),
        output_format: outputFormat(),
        parallelism: parallelism(),
        enable_postprocess: enablePostprocess(),
        groq_model: "whisper-large-v3-turbo",
      };

      await saveSettings(newSettings);
      toastSuccess("Settings saved successfully");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toastError(`Failed to save settings: ${msg}`);
    }
  };

  const handleSaveApiKey = async () => {
    try {
      const key = apiKeyInput().trim();
      if (!key) {
        toastError("API key cannot be empty");
        return;
      }

      await setApiKey(key);
      setApiKeyInput("");
      toastSuccess("API key saved successfully");
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toastError(`Failed to save API key: ${msg}`);
    }
  };

  const handleDeleteApiKey = async () => {
    try {
      if (confirm("Are you sure you want to delete the API key?")) {
        await deleteApiKey();
        toastSuccess("API key deleted");
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      toastError(`Failed to delete API key: ${msg}`);
    }
  };

  return (
    <div style={{ padding: "24px", "max-width": "800px", margin: "0 auto" }}>
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

        <h1 style={{ margin: "0 0 8px 0" }}>Settings</h1>
        <p style={{ margin: "0", color: "#666", "font-size": "14px" }}>Configure Slova</p>
      </div>

      {/* Error message */}
      <Show when={state().error}>
        <div
          style={{
            "background-color": "#fee2e2",
            border: "1px solid #fecaca",
            "border-radius": "4px",
            padding: "12px",
            color: "#991b1b",
            "margin-bottom": "24px",
            "font-size": "14px",
            display: "flex",
            "justify-content": "space-between",
            "align-items": "center",
          }}
        >
          <span>⚠ {state().error}</span>
          <button
            onClick={() => clearError()}
            style={{
              background: "none",
              border: "none",
              color: "#991b1b",
              cursor: "pointer",
              "font-size": "16px",
            }}
          >
            ✕
          </button>
        </div>
      </Show>

      {/* Loading state */}
      <Show when={state().isLoading}>
        <div style={{ "text-align": "center", color: "#666", padding: "24px" }}>
          Loading settings...
        </div>
      </Show>

      <Show when={!state().isLoading}>
        {/* API Key Section */}
        <div
          style={{
            border: "1px solid #e5e7eb",
            "border-radius": "8px",
            padding: "20px",
            "margin-bottom": "24px",
          }}
        >
          <h2 style={{ margin: "0 0 16px 0", "font-size": "16px" }}>🔐 API Key</h2>

          <p style={{ margin: "0 0 8px 0", color: "#666", "font-size": "13px" }}>
            Groq API key for transcription. Stored securely in OS keychain.
          </p>
          <p style={{ margin: "0 0 12px 0", color: "#7c2d12", "font-size": "12px" }}>
            Privacy: audio files are sent to Groq API for transcription.
          </p>

          <div style={{ display: "flex", gap: "8px", "margin-bottom": "12px" }}>
            <input
              type={isShowingApiKey() ? "text" : "password"}
              placeholder="Paste your Groq API key here"
              value={apiKeyInput()}
              onInput={(e) => setApiKeyInput(e.currentTarget.value)}
              style={{
                flex: "1",
                padding: "8px 12px",
                border: "1px solid #d1d5db",
                "border-radius": "4px",
                "font-size": "13px",
                "font-family": "monospace",
              }}
            />
            <button
              onClick={() => setIsShowingApiKey(!isShowingApiKey())}
              style={{
                padding: "8px 12px",
                border: "1px solid #d1d5db",
                "border-radius": "4px",
                "background-color": "#f3f4f6",
                cursor: "pointer",
                "font-size": "13px",
              }}
            >
              {isShowingApiKey() ? "Hide" : "Show"}
            </button>
          </div>

          <div style={{ display: "flex", gap: "8px" }}>
            <button
              onClick={handleSaveApiKey}
              disabled={!apiKeyInput() || state().isSaving}
              style={{
                padding: "8px 16px",
                "background-color": "#10b981",
                color: "white",
                border: "none",
                "border-radius": "4px",
                cursor: "pointer",
                "font-size": "13px",
              }}
            >
              {state().isSaving ? "Saving..." : "Save API Key"}
            </button>

            <Show when={state().apiKeyPresent}>
              <button
                onClick={handleDeleteApiKey}
                disabled={state().isSaving}
                style={{
                  padding: "8px 16px",
                  "background-color": "#ef4444",
                  color: "white",
                  border: "none",
                  "border-radius": "4px",
                  cursor: "pointer",
                  "font-size": "13px",
                }}
              >
                Delete API Key
              </button>
            </Show>
          </div>

          <div
            style={{
              "margin-top": "12px",
              padding: "8px 12px",
              "background-color": state().apiKeyPresent ? "#d1fae5" : "#fee2e2",
              "border-radius": "4px",
              "font-size": "12px",
              color: state().apiKeyPresent ? "#065f46" : "#7c2d12",
            }}
          >
            {state().apiKeyPresent ? "✓ API key is configured" : "✗ No API key set"}
          </div>
        </div>

        {/* Processing Settings */}
        <div
          style={{
            border: "1px solid #e5e7eb",
            "border-radius": "8px",
            padding: "20px",
            "margin-bottom": "24px",
          }}
        >
          <h2 style={{ margin: "0 0 16px 0", "font-size": "16px" }}>⚙ Processing</h2>

          {/* Language */}
          <div style={{ "margin-bottom": "16px" }}>
            <label
              style={{
                display: "block",
                "margin-bottom": "4px",
                "font-weight": "500",
                "font-size": "13px",
              }}
            >
              Language
            </label>
            <select
              value={language()}
              onChange={(e) => setLanguage(e.currentTarget.value)}
              style={{
                width: "100%",
                padding: "8px 12px",
                border: "1px solid #d1d5db",
                "border-radius": "4px",
                "font-size": "13px",
                "box-sizing": "border-box",
              }}
            >
              <option value="ru">Russian</option>
              <option value="en">English</option>
              <option value="es">Spanish</option>
              <option value="fr">French</option>
              <option value="de">German</option>
            </select>
          </div>

          {/* Output Format */}
          <div style={{ "margin-bottom": "16px" }}>
            <label
              style={{
                display: "block",
                "margin-bottom": "4px",
                "font-weight": "500",
                "font-size": "13px",
              }}
            >
              Output Format
            </label>
            <select
              value={outputFormat()}
              onChange={(e) => setOutputFormat(e.currentTarget.value as any)}
              style={{
                width: "100%",
                padding: "8px 12px",
                border: "1px solid #d1d5db",
                "border-radius": "4px",
                "font-size": "13px",
                "box-sizing": "border-box",
              }}
            >
              <option value="txt">Plain Text (.txt)</option>
              <option value="srt">Subtitles (.srt)</option>
              <option value="json">JSON (.json)</option>
            </select>
          </div>

          {/* Parallelism */}
          <div style={{ "margin-bottom": "16px" }}>
            <label
              style={{
                display: "block",
                "margin-bottom": "4px",
                "font-weight": "500",
                "font-size": "13px",
              }}
            >
              Concurrent Jobs
            </label>
            <div style={{ display: "flex", "align-items": "center", gap: "8px" }}>
              <input
                type="range"
                min="1"
                max="10"
                value={parallelism()}
                onChange={(e) => setParallelism(parseInt(e.currentTarget.value))}
                style={{ flex: "1" }}
              />
              <span
                style={{
                  "min-width": "30px",
                  "text-align": "right",
                  "font-weight": "500",
                }}
              >
                {parallelism()}
              </span>
            </div>
            <p style={{ margin: "4px 0 0 0", color: "#666", "font-size": "12px" }}>
              Number of jobs to process simultaneously. Higher values = faster but more CPU usage.
            </p>
          </div>

          {/* Postprocessing */}
          <div style={{ "margin-bottom": "16px" }}>
            <label
              style={{
                display: "flex",
                "align-items": "center",
                gap: "8px",
                cursor: "pointer",
                "font-size": "13px",
              }}
            >
              <input
                type="checkbox"
                checked={enablePostprocess()}
                onChange={(e) => setEnablePostprocess(e.currentTarget.checked)}
              />
              <span>Enable postprocessing with Llama</span>
            </label>
            <p style={{ margin: "4px 0 0 24px", color: "#666", "font-size": "12px" }}>
              Clean up punctuation and grammar. Adds ~1 second per file.
            </p>
          </div>
        </div>

        {/* Save Button */}
        <div style={{ display: "flex", gap: "8px" }}>
          <button
            onClick={handleSaveSettings}
            disabled={state().isSaving}
            style={{
              padding: "10px 20px",
              "background-color": "#4f46e5",
              color: "white",
              border: "none",
              "border-radius": "4px",
              cursor: "pointer",
              "font-size": "13px",
              "font-weight": "500",
            }}
          >
            {state().isSaving ? "Saving..." : "Save Settings"}
          </button>

          <button
            onClick={() => navigate("/")}
            style={{
              padding: "10px 20px",
              "background-color": "#f3f4f6",
              border: "1px solid #d1d5db",
              "border-radius": "4px",
              cursor: "pointer",
              "font-size": "13px",
            }}
          >
            Cancel
          </button>
        </div>
      </Show>
    </div>
  );
}
