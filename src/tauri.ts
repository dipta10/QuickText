import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { BackendAppSnapshot, PartialTranscriptUpdate } from "./app-state";

const getAppStateCommand = "get_app_state";
const toggleRecordingCommand = "toggle_recording";
const setGlobalShortcutCommand = "set_global_shortcut";
const setShortcutBehaviorCommand = "set_shortcut_behavior";
const hasSonioxApiKeyCommand = "has_soniox_api_key";
const saveSonioxApiKeyCommand = "save_soniox_api_key";
const deleteSonioxApiKeyCommand = "delete_soniox_api_key";
const copyTextToClipboardCommand = "copy_text_to_clipboard";
const getLaunchOnStartupCommand = "get_launch_on_startup";
const setLaunchOnStartupCommand = "set_launch_on_startup";
const getLoggingStatusCommand = "get_logging_status";
const enableTemporaryDebugLoggingCommand = "enable_temporary_debug_logging";
const exportLogsCommand = "export_logs";
const deleteLocalLogsCommand = "delete_local_logs";
const appStateChangedEvent = "app-state-changed";
const partialTranscriptEvent = "partial-transcript";

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

export const getLaunchOnStartup = async (): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(getLaunchOnStartupCommand);
};

export const setLaunchOnStartup = async (
  enabled: boolean,
): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(setLaunchOnStartupCommand, { enabled });
};

export type LoggingStatus = {
  debugSecondsRemaining: number;
};

export const getLoggingStatus = async (): Promise<LoggingStatus> => {
  assertTauriRuntime();
  return await invoke<LoggingStatus>(getLoggingStatusCommand);
};

export const enableTemporaryDebugLogging = async (): Promise<LoggingStatus> => {
  assertTauriRuntime();
  return await invoke<LoggingStatus>(enableTemporaryDebugLoggingCommand);
};

export const exportLogs = async (): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(exportLogsCommand);
};

export const deleteLocalLogs = async () => {
  assertTauriRuntime();
  await invoke(deleteLocalLogsCommand);
};

export const copyTextToClipboard = async (text: string) => {
  assertTauriRuntime();
  await invoke(copyTextToClipboardCommand, { text });
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
