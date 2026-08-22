import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { BackendAppSnapshot } from "./app-state";

const getAppStateCommand = "get_app_state";
const toggleRecordingCommand = "toggle_recording";
const setGlobalShortcutCommand = "set_global_shortcut";
const hasSonioxApiKeyCommand = "has_soniox_api_key";
const saveSonioxApiKeyCommand = "save_soniox_api_key";
const deleteSonioxApiKeyCommand = "delete_soniox_api_key";
const globalShortcutPressedEvent = "global-shortcut-pressed";
const appStateChangedEvent = "app-state-changed";

const assertTauriRuntime = () => {
  if (!("__TAURI_INTERNALS__" in window)) {
    throw new Error("Run the desktop app with npm run tauri dev.");
  }
};

export const setGlobalShortcut = async (shortcut: string) => {
  assertTauriRuntime();
  await invoke(setGlobalShortcutCommand, { shortcut });
};

export const getAppState = async (): Promise<BackendAppSnapshot> => {
  assertTauriRuntime();
  return await invoke<BackendAppSnapshot>(getAppStateCommand);
};

export const toggleBackendRecording = async (
  maxRecordingSeconds: number,
): Promise<BackendAppSnapshot> => {
  assertTauriRuntime();
  return await invoke<BackendAppSnapshot>(toggleRecordingCommand, {
    maxRecordingSeconds,
  });
};

export const hasSonioxApiKey = async (): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(hasSonioxApiKeyCommand);
};

export const saveSonioxApiKey = async (apiKey: string): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(saveSonioxApiKeyCommand, { apiKey });
};

export const deleteSonioxApiKey = async () => {
  assertTauriRuntime();
  await invoke(deleteSonioxApiKeyCommand);
};

export const copyTextToClipboard = async (text: string) => {
  assertTauriRuntime();
  await writeText(text);
};

export const onGlobalShortcutPressed = (handler: () => void) => {
  assertTauriRuntime();
  return listen<string>(globalShortcutPressedEvent, () => {
    handler();
  });
};

export const onAppStateChanged = (
  handler: (snapshot: BackendAppSnapshot) => void,
) => {
  assertTauriRuntime();
  return listen<BackendAppSnapshot>(appStateChangedEvent, (event) => {
    handler(event.payload);
  });
};
