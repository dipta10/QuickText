import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type { BackendAppSnapshot, PartialTranscriptUpdate } from "./app-state";

const getAppStateCommand = "get_app_state";
const toggleRecordingCommand = "toggle_recording";
const setGlobalShortcutCommand = "set_global_shortcut";
const setShortcutBehaviorCommand = "set_shortcut_behavior";
const hasSonioxApiKeyCommand = "has_soniox_api_key";
const saveSonioxApiKeyCommand = "save_soniox_api_key";
const deleteSonioxApiKeyCommand = "delete_soniox_api_key";
const listInputDevicesCommand = "list_input_devices";
const setInputDeviceCommand = "set_input_device";
const appStateChangedEvent = "app-state-changed";
const partialTranscriptEvent = "partial-transcript";
const deviceFallbackEvent = "device-fallback";

export type BackendInputDeviceInfo = {
  id: string;
  label: string;
};

export type BackendInputDeviceList = {
  defaultLabel: string | null;
  devices: BackendInputDeviceInfo[];
};

const assertTauriRuntime = () => {
  if (!("__TAURI_INTERNALS__" in window)) {
    throw new Error("Run the desktop app with npm run tauri dev.");
  }
};

export const setGlobalShortcut = async (shortcut: string) => {
  assertTauriRuntime();
  await invoke(setGlobalShortcutCommand, { shortcut });
};

export const setShortcutBehavior = async (
  focusOnStart: boolean,
  hideOnStop: boolean,
) => {
  assertTauriRuntime();
  await invoke(setShortcutBehaviorCommand, { focusOnStart, hideOnStop });
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

export const listInputDevices = async (): Promise<BackendInputDeviceList> => {
  assertTauriRuntime();
  return await invoke<BackendInputDeviceList>(listInputDevicesCommand);
};

export const setInputDevice = async (deviceId: string | null) => {
  assertTauriRuntime();
  await invoke(setInputDeviceCommand, { deviceId });
};

export const copyTextToClipboard = async (text: string) => {
  assertTauriRuntime();
  await writeText(text);
};

export const onAppStateChanged = (
  handler: (snapshot: BackendAppSnapshot) => void,
) => {
  assertTauriRuntime();
  return listen<BackendAppSnapshot>(appStateChangedEvent, (event) => {
    handler(event.payload);
  });
};

export const onPartialTranscript = (
  handler: (update: PartialTranscriptUpdate) => void,
) => {
  assertTauriRuntime();
  return listen<PartialTranscriptUpdate>(partialTranscriptEvent, (event) => {
    handler(event.payload);
  });
};

export const onDeviceFallback = (handler: (message: string) => void) => {
  assertTauriRuntime();
  return listen<string>(deviceFallbackEvent, (event) => {
    handler(event.payload);
  });
};
