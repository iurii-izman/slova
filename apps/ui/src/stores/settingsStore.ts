import { createStore } from "solid-js/store";
import type { Settings } from "../ipc/types";
import * as commands from "../ipc/commands";

export type SettingsStore = {
  settings: Settings | null;
  apiKeyPresent: boolean;
  isLoading: boolean;
  isSaving: boolean;
  error: string | null;
};

// Initial settings with defaults
const defaultSettings: Settings = {
  language: "ru",
  output_format: "txt",
  parallelism: 3,
  enable_postprocess: false,
  groq_model: "whisper-large-v3-turbo",
};

const [settingsState, setSettingsState] = createStore<SettingsStore>({
  settings: defaultSettings,
  apiKeyPresent: false,
  isLoading: false,
  isSaving: false,
  error: null,
});

/**
 * Load settings from backend
 */
export async function loadSettings() {
  try {
    setSettingsState("isLoading", true);
    setSettingsState("error", null);

    const loaded = await commands.getSettings();
    setSettingsState("settings", loaded);

    // Check if API key is present
    const hasKey = await commands.checkApiKey();
    setSettingsState("apiKeyPresent", hasKey);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    setSettingsState("error", message);
    console.error("Failed to load settings:", err);
  } finally {
    setSettingsState("isLoading", false);
  }
}

/**
 * Save settings to backend
 */
export async function saveSettings(newSettings: Settings) {
  try {
    setSettingsState("isSaving", true);
    setSettingsState("error", null);

    await commands.setSettings(newSettings);
    setSettingsState("settings", newSettings);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    setSettingsState("error", message);
    throw err;
  } finally {
    setSettingsState("isSaving", false);
  }
}

/**
 * Save API key
 */
export async function setApiKey(key: string) {
  try {
    setSettingsState("isSaving", true);
    setSettingsState("error", null);

    await commands.saveApiKey(key);
    setSettingsState("apiKeyPresent", true);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    setSettingsState("error", message);
    throw err;
  } finally {
    setSettingsState("isSaving", false);
  }
}

/**
 * Delete API key
 */
export async function deleteApiKey() {
  try {
    setSettingsState("isSaving", true);
    setSettingsState("error", null);

    await commands.deleteApiKey();
    setSettingsState("apiKeyPresent", false);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    setSettingsState("error", message);
    throw err;
  } finally {
    setSettingsState("isSaving", false);
  }
}

/**
 * Get current settings
 */
export function getSettings(): Settings | null {
  return settingsState.settings;
}

/**
 * Get settings state
 */
export function getSettingsState() {
  return settingsState;
}

/**
 * Clear error
 */
export function clearError() {
  setSettingsState("error", null);
}
