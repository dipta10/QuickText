export type RecordingState =
  | "idle"
  | "starting"
  | "recording"
  | "stopping"
  | "transcribed"
  | "error";
export type ShortcutCaptureState = "idle" | "capturing";
export type ActiveView = "capture" | "settings";

export type TranscriptResult = {
  text: string;
  provider: string;
};

export type BackendAppError = {
  type: string;
  message: string;
  supportReference: string;
};

export type BackendAppSnapshot = {
  status: RecordingState;
  transcript: TranscriptResult | null;
  error: BackendAppError | null;
};

export type PartialTranscriptUpdate = {
  final_text: string;
  partial_text: string;
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
  launchOnStartup: boolean;
  launchOnStartupStatus: string;
  hasApiKey: boolean;
  apiKeyStatus: string;
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
  launchOnStartup,
  launchOnStartupStatus: "",
  hasApiKey: false,
  apiKeyStatus: "",
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
    return `${snapshot.error.message} Support reference: ${snapshot.error.supportReference}`;
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
    status: statusText(snapshot),
    transcript: snapshot.transcript?.text ?? state.transcript,
    partialTranscript:
      snapshot.status === "recording" ? state.partialTranscript : "",
    recordingStartedAt: recordingStartedAt(state, snapshot),
    apiKeyStatus: isMissingApiKey
      ? statusText(snapshot)
      : state.apiKeyStatus,
  };
};

export const applyPartialTranscript = (
  state: AppState,
  update: PartialTranscriptUpdate,
): AppState => {
  if (state.recording !== "recording") {
    return state;
  }

  return {
    ...state,
    transcript: state.liveTranscript ? update.final_text : state.transcript,
    partialTranscript:
      state.liveTranscript && state.showPartialTranscript
        ? update.partial_text
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

export const setApiKeyPresence = (
  state: AppState,
  hasApiKey: boolean,
  apiKeyStatus = "",
): AppState => ({
  ...state,
  hasApiKey,
  apiKeyStatus,
});

export const setApiKeyStatus = (
  state: AppState,
  apiKeyStatus: string,
): AppState => ({
  ...state,
  apiKeyStatus,
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
