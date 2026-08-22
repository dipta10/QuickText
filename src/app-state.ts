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
};

export type BackendAppSnapshot = {
  status: RecordingState;
  transcript: TranscriptResult | null;
  error: BackendAppError | null;
};

export type AppState = {
  activeView: ActiveView;
  recording: RecordingState;
  shortcutCapture: ShortcutCaptureState;
  selectedShortcut: string;
  hasApiKey: boolean;
  apiKeyStatus: string;
  status: string;
  transcript: string;
  recordingStartedAt: number | null;
};

export const createAppState = (selectedShortcut: string): AppState => ({
  activeView: "capture",
  recording: "idle",
  shortcutCapture: "idle",
  selectedShortcut,
  hasApiKey: false,
  apiKeyStatus: "",
  status: "Ready.",
  transcript: "",
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

  return {
    ...state,
    activeView: isMissingApiKey ? "settings" : state.activeView,
    recording: snapshot.status,
    status: statusText(snapshot),
    transcript: snapshot.transcript?.text ?? state.transcript,
    recordingStartedAt: recordingStartedAt(state, snapshot),
    apiKeyStatus: isMissingApiKey
      ? snapshot.error?.message ?? state.apiKeyStatus
      : state.apiKeyStatus,
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

export const setStatus = (state: AppState, status: string): AppState => ({
  ...state,
  status,
});
