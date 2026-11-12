import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";
import type {
  BackendAppSnapshot,
  AudioLevelUpdate,
  PartialTranscriptUpdate,
  ProviderId,
} from "./app-state";

const getAppStateCommand = "get_app_state";
const getBuildInfoCommand = "get_build_info";
const toggleRecordingCommand = "toggle_recording";
const setWindowShortcutCommand = "set_window_shortcut";
const setGlobalShortcutCommand = "set_global_shortcut";
const initializeShortcutsCommand = "initialize_shortcuts";
const setShortcutBehaviorCommand = "set_shortcut_behavior";
const setPasteToTargetCommand = "set_paste_to_target";
const hasProviderApiKeyCommand = "has_provider_api_key";
const saveProviderApiKeyCommand = "save_provider_api_key";
const deleteProviderApiKeyCommand = "delete_provider_api_key";
const getTranscriptionSettingsCommand = "get_transcription_settings";
const setActiveProviderCommand = "set_active_provider";
const listInputDevicesCommand = "list_input_devices";
const setInputDeviceCommand = "set_input_device";
const getLaunchOnStartupCommand = "get_launch_on_startup";
const setLaunchOnStartupCommand = "set_launch_on_startup";
const getLanguagePreferencesCommand = "get_language_preferences";
const setLanguagePreferencesCommand = "set_language_preferences";
const getTranscriptionDescriptionCommand = "get_transcription_description";
const setTranscriptionDescriptionCommand = "set_transcription_description";
const getTranscriptionTermsCommand = "get_transcription_terms";
const setTranscriptionTermsCommand = "set_transcription_terms";
const appStateChangedEvent = "app-state-changed";
const partialTranscriptEvent = "partial-transcript";
const deviceFallbackEvent = "device-fallback";
const audioLevelEvent = "audio-level";

export type BackendInputDeviceInfo = {
  id: string;
  label: string;
};

export type BackendInputDeviceList = {
  defaultLabel: string | null;
  devices: BackendInputDeviceInfo[];
};

export type BuildInfo = {
  version: string;
  buildId: string;
  sourceRevision: string;
};

export type SupportedLanguage = {
  code: string;
  name: string;
};

export type LanguagePreferencesSnapshot = {
  availableLanguages: SupportedLanguage[];
  selectedCodes: string[];
};

export type TranscriptionSettingsSnapshot = {
  activeProvider: ProviderId;
};

const assertTauriRuntime = () => {
  if (!("__TAURI_INTERNALS__" in window)) {
    throw new Error("Run the desktop app with npm run tauri dev.");
  }
};

export type ShortcutSettingsSnapshot = {
  recording: string;
  window: string;
  recordingError: string | null;
  windowError: string | null;
  speech?: string;
  speechStop?: string;
  speechError?: string | null;
  speechStopError?: string | null;
};

export const initializeShortcuts = async (
  legacyRecording: string,
  legacyWindow: string,
): Promise<ShortcutSettingsSnapshot> => {
  assertTauriRuntime();
  return invoke(initializeShortcutsCommand, { legacyRecording, legacyWindow });
};

export const setGlobalShortcut = async (shortcut: string) => {
  assertTauriRuntime();
  await invoke(setGlobalShortcutCommand, { shortcut });
};

export const setWindowShortcut = async (shortcut: string) => {
  assertTauriRuntime();
  await invoke(setWindowShortcutCommand, { shortcut });
};

export const setShortcutBehavior = async (
  focusOnStart: boolean,
  hideOnStop: boolean,
) => {
  assertTauriRuntime();
  await invoke(setShortcutBehaviorCommand, { focusOnStart, hideOnStop });
};

export const setPasteToTargetBackend = async (enabled: boolean) => {
  assertTauriRuntime();
  await invoke(setPasteToTargetCommand, { enabled });
};

export const getAppState = async (): Promise<BackendAppSnapshot> => {
  assertTauriRuntime();
  return await invoke<BackendAppSnapshot>(getAppStateCommand);
};

export const getBuildInfo = async (): Promise<BuildInfo> => {
  assertTauriRuntime();
  return await invoke<BuildInfo>(getBuildInfoCommand);
};

export const toggleBackendRecording = async (
  maxRecordingSeconds: number,
): Promise<BackendAppSnapshot> => {
  assertTauriRuntime();
  return await invoke<BackendAppSnapshot>(toggleRecordingCommand, {
    maxRecordingSeconds,
  });
};

export const hasProviderApiKey = async (
  provider: ProviderId,
): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(hasProviderApiKeyCommand, { provider });
};

export const saveProviderApiKey = async (
  provider: ProviderId,
  apiKey: string,
): Promise<boolean> => {
  assertTauriRuntime();
  return await invoke<boolean>(saveProviderApiKeyCommand, { provider, apiKey });
};

export const deleteProviderApiKey = async (provider: ProviderId) => {
  assertTauriRuntime();
  await invoke(deleteProviderApiKeyCommand, { provider });
};

export const getTranscriptionSettings = async (): Promise<TranscriptionSettingsSnapshot> => {
  assertTauriRuntime();
  return await invoke<TranscriptionSettingsSnapshot>(getTranscriptionSettingsCommand);
};

export const setActiveProvider = async (
  provider: ProviderId,
): Promise<ProviderId> => {
  assertTauriRuntime();
  return await invoke<ProviderId>(setActiveProviderCommand, { provider });
};

export const listInputDevices = async (): Promise<BackendInputDeviceList> => {
  assertTauriRuntime();
  return await invoke<BackendInputDeviceList>(listInputDevicesCommand);
};

export const setInputDevice = async (deviceId: string | null) => {
  assertTauriRuntime();
  await invoke(setInputDeviceCommand, { deviceId });
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

export const getLanguagePreferences = async (): Promise<LanguagePreferencesSnapshot> => {
  assertTauriRuntime();
  return await invoke<LanguagePreferencesSnapshot>(getLanguagePreferencesCommand);
};

export const setLanguagePreferences = async (
  selectedCodes: string[],
): Promise<string[]> => {
  assertTauriRuntime();
  return await invoke<string[]>(setLanguagePreferencesCommand, { selectedCodes });
};

export const getTranscriptionDescription = async (): Promise<string> => {
  assertTauriRuntime();
  return await invoke<string>(getTranscriptionDescriptionCommand);
};

export const setTranscriptionDescription = async (
  description: string,
): Promise<string> => {
  assertTauriRuntime();
  return await invoke<string>(setTranscriptionDescriptionCommand, {
    description,
  });
};

export const getTranscriptionTerms = async (): Promise<string> => {
  assertTauriRuntime();
  return await invoke<string>(getTranscriptionTermsCommand);
};

export const setTranscriptionTerms = async (
  termsText: string,
): Promise<string> => {
  assertTauriRuntime();
  return await invoke<string>(setTranscriptionTermsCommand, { termsText });
};

export const copyTextToClipboard = async (text: string) => {
  assertTauriRuntime();
  await writeText(text);
};

export const onAppStateChanged = async (
  handler: (snapshot: BackendAppSnapshot) => void,
) => {
  assertTauriRuntime();
  return listen<BackendAppSnapshot>(appStateChangedEvent, (event) => {
    handler(event.payload);
  });
};

export const onPartialTranscript = async (
  handler: (update: PartialTranscriptUpdate) => void,
) => {
  assertTauriRuntime();
  return listen<PartialTranscriptUpdate>(partialTranscriptEvent, (event) => {
    handler(event.payload);
  });
};

export const onDeviceFallback = async (handler: (message: string) => void) => {
  assertTauriRuntime();
  return listen<string>(deviceFallbackEvent, (event) => {
    handler(event.payload);
  });
};

export const onAudioLevel = async (handler: (update: AudioLevelUpdate) => void) => {
  assertTauriRuntime();
  return listen<AudioLevelUpdate>(audioLevelEvent, (event) => {
    handler(event.payload);
  });
};

export const hasCleanupApiKey = async (): Promise<boolean> => {
  assertTauriRuntime();
  return invoke<boolean>("has_cleanup_api_key");
};

export const saveCleanupApiKey = async (apiKey: string): Promise<void> => {
  assertTauriRuntime();
  await invoke("save_cleanup_api_key", { apiKey });
};

export const deleteCleanupApiKey = async (): Promise<void> => {
  assertTauriRuntime();
  await invoke("delete_cleanup_api_key");
};

export const cleanAndCopy = async (): Promise<void> => {
  assertTauriRuntime();
  await invoke("clean_and_copy");
};

export const restoreOriginalTranscript = async (): Promise<void> => {
  assertTauriRuntime();
  await invoke("restore_original_transcript");
};

export const openCleanupKeySetup = async (): Promise<void> => {
  assertTauriRuntime();
  await invoke("open_cleanup_key_setup");
};

export type SpeechSettings = { voice: string; language: string; speed: number };
export type SpeechSnapshot = {
  status: "idle" | "preparing" | "playing" | "completed" | "error";
  playbackSpeed?: number;
  sessionId: number;
  revision: number;
  error: string | null;
};
export const getSpeechState = async (): Promise<SpeechSnapshot> => {
  assertTauriRuntime();
  return invoke("get_speech_state");
};
export const getSpeechSettings = async (): Promise<SpeechSettings> => {
  assertTauriRuntime();
  return invoke("get_speech_settings");
};
export const setSpeechSettings = async (settings: SpeechSettings): Promise<SpeechSettings> => {
  assertTauriRuntime();
  return invoke("set_speech_settings", { settings });
};
export const toggleSpeech = async (text: string | null): Promise<SpeechSnapshot> => {
  assertTauriRuntime();
  return invoke("toggle_speech", { text });
};
export const stopSpeech = async (): Promise<SpeechSnapshot> => {
  assertTauriRuntime();
  return invoke("stop_speech");
};
export const setSpeechShortcut = async (shortcut: string, stopOnly: boolean): Promise<string> => {
  assertTauriRuntime();
  return invoke("set_speech_shortcut", { shortcut, stopOnly });
};
export const onSpeechStateChanged = async (handler: (snapshot: SpeechSnapshot) => void) => {
  assertTauriRuntime();
  return listen<SpeechSnapshot>("speech-state-changed", (event) => handler(event.payload));
};
export const readClipboardText = async (): Promise<string> => {
  assertTauriRuntime();
  return readText();
};

export const setPlaybackSpeed = async (speed: number): Promise<SpeechSnapshot> => {
  assertTauriRuntime();
  return invoke("set_playback_speed", { speed });
};

export type SpeechVoice = { id: string; description: string; gender: string };
export const listSpeechVoices = async (): Promise<SpeechVoice[]> => {
  assertTauriRuntime();
  return invoke("list_speech_voices");
};
