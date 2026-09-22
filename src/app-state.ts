export type RecordingState =
  | "idle"
  | "starting"
  | "recording"
  | "stopping"
  | "transcribed"
  | "error";
export type ShortcutCaptureState = "idle" | "capturing";
export type ActiveView = "capture" | "settings";
export type ProviderId = "soniox" | "deepgram";

export type TranscriptResult = {
  text: string;
  provider: string;
};

export type BackendAppError = {
  type: string;
  provider?: ProviderId;
  message: string;
};

export type BackendAppSnapshot = {
  status: RecordingState;
  sessionId: number | null;
  transcript: TranscriptResult | null;
  error: BackendAppError | null;
};

export type PartialTranscriptUpdate = {
  recordingSessionId: number;
  providerSessionId: number;
  finalText: string;
  partialText: string;
};

export type InputDeviceOption = {
  id: string;
  label: string;
};

export type AppState = {
  activeView: ActiveView;
  recording: RecordingState;
  shortcutCapture: ShortcutCaptureState;
  selectedShortcut: string;
  shortcutFocusOnStart: boolean;
  shortcutHideOnStop: boolean;
  maxRecordingSeconds: number;
  autoCopyTranscript: boolean;
  liveTranscript: boolean;
  showPartialTranscript: boolean;
  pasteToTarget: boolean;
  launchOnStartup: boolean;
  launchOnStartupStatus: string;
  activeProvider: ProviderId;
  activeSessionId: number | null;
  providerApiKeyConfigured: Record<ProviderId, boolean>;
  providerApiKeyStatus: Record<ProviderId, string>;
  inputDevices: InputDeviceOption[];
  inputDeviceDefaultLabel: string;
  selectedInputDeviceId: string;
  deviceNotice: string;
  status: string;
  transcript: string;
  partialTranscript: string;
  recordingStartedAt: number | null;
};

export const createAppState = (
  selectedShortcut: string,
  maxRecordingSeconds: number,
  autoCopyTranscript: boolean,
  shortcutFocusOnStart: boolean,
  shortcutHideOnStop: boolean,
  liveTranscript: boolean,
  showPartialTranscript: boolean,
  selectedInputDeviceId: string,
  pasteToTarget: boolean,
  launchOnStartup: boolean,
): AppState => ({
  activeView: "capture",
  recording: "idle",
  shortcutCapture: "idle",
  selectedShortcut,
  shortcutFocusOnStart,
  shortcutHideOnStop,
  maxRecordingSeconds,
  autoCopyTranscript,
  liveTranscript,
  showPartialTranscript,
  pasteToTarget,
  launchOnStartup,
  launchOnStartupStatus: "",
  activeProvider: "soniox",
  activeSessionId: null,
  providerApiKeyConfigured: { soniox: false, deepgram: false },
  providerApiKeyStatus: { soniox: "", deepgram: "" },
  inputDevices: [],
  inputDeviceDefaultLabel: "",
  selectedInputDeviceId,
  deviceNotice: "",
  status: "Ready.",
  transcript: "",
  partialTranscript: "",
  recordingStartedAt: null,
});

export const isRecording = (state: AppState) => state.recording === "recording";
export const isBusy = (state: AppState) =>
  state.recording === "starting" || state.recording === "stopping";

export const isCapturingShortcut = (state: AppState) =>
  state.shortcutCapture === "capturing";

export const startShortcutCapture = (state: AppState): AppState => ({
  ...state,
  shortcutCapture: "capturing",
  status: "Press a shortcut. Esc cancels.",
});

export const cancelShortcutCapture = (state: AppState): AppState => ({
  ...state,
  shortcutCapture: "idle",
  status: "Canceled.",
});

const statusText = (snapshot: BackendAppSnapshot): string => {
  if (snapshot.error) {
    return snapshot.error.message;
  }

  switch (snapshot.status) {
    case "idle":
      return "Ready.";
    case "starting":
      return "Starting recording...";
    case "recording":
      return "Recording.";
    case "stopping":
      return "Finalizing transcription...";
    case "transcribed":
      return "Transcript ready.";
    case "error":
      return "Recording failed.";
  }
};

const recordingStartedAt = (
  state: AppState,
  snapshot: BackendAppSnapshot,
): number | null => {
  if (snapshot.status === "recording") {
    return state.recordingStartedAt ?? Date.now();
  }

  return null;
};

export const applyBackendSnapshot = (
  state: AppState,
  snapshot: BackendAppSnapshot,
): AppState => {
  const isMissingApiKey = snapshot.error?.type === "missing_api_key";
  const shouldClearTranscript =
    snapshot.status === "starting" || snapshot.status === "error";
  const shouldShowCapture =
    snapshot.status !== "idle" && snapshot.status !== "error";

  return {
    ...state,
    activeView: isMissingApiKey
      ? "settings"
      : shouldShowCapture
        ? "capture"
        : state.activeView,
    recording: snapshot.status,
    activeSessionId: snapshot.sessionId,
    status: statusText(snapshot),
    transcript:
      snapshot.transcript?.text ??
      (shouldClearTranscript ? "" : state.transcript),
    partialTranscript:
      snapshot.status === "recording" ? state.partialTranscript : "",
    recordingStartedAt: recordingStartedAt(state, snapshot),
    providerApiKeyStatus: isMissingApiKey
      ? {
          ...state.providerApiKeyStatus,
          [snapshot.error?.provider ?? state.activeProvider]:
            snapshot.error?.message ??
            state.providerApiKeyStatus[
              snapshot.error?.provider ?? state.activeProvider
            ],
        }
      : state.providerApiKeyStatus,
    deviceNotice:
      snapshot.status === "starting" ? "" : state.deviceNotice,
  };
};

export const setDeviceNotice = (
  state: AppState,
  deviceNotice: string,
): AppState => ({
  ...state,
  deviceNotice,
});

export const applyPartialTranscript = (
  state: AppState,
  update: PartialTranscriptUpdate,
): AppState => {
  if (
    state.recording !== "recording" ||
    state.activeSessionId !== update.recordingSessionId
  ) {
    return state;
  }

  return {
    ...state,
    transcript: state.liveTranscript ? update.finalText : state.transcript,
    partialTranscript:
      state.liveTranscript && state.showPartialTranscript
        ? update.partialText
        : "",
  };
};

export const showCapture = (state: AppState): AppState => ({
  ...state,
  activeView: "capture",
});

export const showSettings = (state: AppState): AppState => ({
  ...state,
  activeView: "settings",
});

export const saveShortcut = (
  state: AppState,
  selectedShortcut: string,
): AppState => ({
  ...state,
  shortcutCapture: "idle",
  selectedShortcut,
  status: "Saved.",
});

export const setActiveProvider = (
  state: AppState,
  activeProvider: ProviderId,
): AppState => ({
  ...state,
  activeProvider,
});

export const setProviderApiKeyPresence = (
  state: AppState,
  provider: ProviderId,
  hasApiKey: boolean,
  apiKeyStatus = "",
): AppState => ({
  ...state,
  providerApiKeyConfigured: {
    ...state.providerApiKeyConfigured,
    [provider]: hasApiKey,
  },
  providerApiKeyStatus: {
    ...state.providerApiKeyStatus,
    [provider]: apiKeyStatus,
  },
});

export const setProviderApiKeyStatus = (
  state: AppState,
  provider: ProviderId,
  apiKeyStatus: string,
): AppState => ({
  ...state,
  providerApiKeyStatus: {
    ...state.providerApiKeyStatus,
    [provider]: apiKeyStatus,
  },
});

export const setInputDeviceOptions = (
  state: AppState,
  inputDevices: InputDeviceOption[],
  inputDeviceDefaultLabel: string,
): AppState => ({
  ...state,
  inputDevices,
  inputDeviceDefaultLabel,
});

export const setSelectedInputDevice = (
  state: AppState,
  selectedInputDeviceId: string,
): AppState => ({
  ...state,
  selectedInputDeviceId,
  status: selectedInputDeviceId
    ? `Microphone set to "${selectedInputDeviceId}".`
    : "Using the system default microphone.",
});

export const setMaxRecordingSeconds = (
  state: AppState,
  maxRecordingSeconds: number,
): AppState => ({
  ...state,
  maxRecordingSeconds,
  status: `Maximum recording set to ${maxRecordingSeconds} seconds.`,
});

export const setAutoCopyTranscript = (
  state: AppState,
  autoCopyTranscript: boolean,
): AppState => ({
  ...state,
  autoCopyTranscript,
  status: autoCopyTranscript
    ? "Auto-copy enabled."
    : "Auto-copy disabled.",
});

export const setLiveTranscript = (
  state: AppState,
  liveTranscript: boolean,
): AppState => ({
  ...state,
  liveTranscript,
  status: liveTranscript
    ? "Real-time transcript enabled."
    : "Real-time transcript disabled.",
});

export const setShowPartialTranscript = (
  state: AppState,
  showPartialTranscript: boolean,
): AppState => ({
  ...state,
  showPartialTranscript,
  status: showPartialTranscript
    ? "Unconfirmed words will be shown while recording."
    : "Unconfirmed words will be hidden.",
});

export const setShortcutFocusOnStart = (
  state: AppState,
  shortcutFocusOnStart: boolean,
): AppState => ({
  ...state,
  shortcutFocusOnStart,
  status: shortcutFocusOnStart
    ? "Shortcut will focus the window when recording starts."
    : "Shortcut will start recording without focusing the window.",
});

export const setShortcutHideOnStop = (
  state: AppState,
  shortcutHideOnStop: boolean,
): AppState => ({
  ...state,
  shortcutHideOnStop,
  status: shortcutHideOnStop
    ? "Shortcut will hide the window when recording stops."
    : "Shortcut will keep the window open when recording stops.",
});

export const setPasteToTarget = (
  state: AppState,
  pasteToTarget: boolean,
): AppState => ({
  ...state,
  pasteToTarget,
  status: pasteToTarget
    ? "Transcripts will paste into the focused app."
    : "Paste-to-target disabled.",
});

export const setLaunchOnStartup = (
  state: AppState,
  launchOnStartup: boolean,
  launchOnStartupStatus = "",
): AppState => ({
  ...state,
  launchOnStartup,
  launchOnStartupStatus,
});

export const setLaunchOnStartupStatus = (
  state: AppState,
  launchOnStartupStatus: string,
): AppState => ({
  ...state,
  launchOnStartupStatus,
});

export const setStatus = (state: AppState, status: string): AppState => ({
  ...state,
  status,
});
