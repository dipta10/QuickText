import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const setGlobalShortcutCommand = "set_global_shortcut";
const hasSonioxApiKeyCommand = "has_soniox_api_key";
const saveSonioxApiKeyCommand = "save_soniox_api_key";
const deleteSonioxApiKeyCommand = "delete_soniox_api_key";
const globalShortcutPressedEvent = "global-shortcut-pressed";

const assertTauriRuntime = () => {
  if (!("__TAURI_INTERNALS__" in window)) {
    throw new Error("Run the desktop app with npm run tauri dev.");
  }
};

export const setGlobalShortcut = async (shortcut: string) => {
  assertTauriRuntime();
  await invoke(setGlobalShortcutCommand, { shortcut });
};

export const hasSonioxApiKey = async (): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(hasSonioxApiKeyCommand);
};

export const saveSonioxApiKey = async (apiKey: string) => {
  assertTauriRuntime();
  await invoke(saveSonioxApiKeyCommand, { apiKey });
};

export const deleteSonioxApiKey = async () => {
  assertTauriRuntime();
  await invoke(deleteSonioxApiKeyCommand);
};

export const onGlobalShortcutPressed = (handler: () => void) => {
  assertTauriRuntime();
  return listen<string>(globalShortcutPressedEvent, () => {
    handler();
  });
};
