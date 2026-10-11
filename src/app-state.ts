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
export type ProviderSelectionStatus = "loading" | "ready" | "error";

export type TranscriptResult = {
  text: string;
  provider: string;
};

export type BackendAppError = {
  type: string;
  provider?: ProviderId;
  message: string;
};

export type CleanupSnapshot = {
  revision: number;
  running: boolean;
  cleaned: boolean;
  message: string;
};

const emptyCleanup = (): CleanupSnapshot => ({
  revision: 0, running: false, cleaned: false, message: "",
});

export type BackendAppSnapshot = {
  status: RecordingState;
  sessionId: number | null;
  transcript: TranscriptResult | null;
  error: BackendAppError | null;
  cleanup?: CleanupSnapshot;
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
  selectedWindowShortcut: string;
  shortcutCaptureTarget: "recording" | "window";
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
  providerSelectionStatus: ProviderSelectionStatus;
  activeSessionId: number | null;
  providerApiKeyConfigured: Record<ProviderId, boolean>;
  providerApiKeyStatus: Record<ProviderId, string>;
  inputDevices: InputDeviceOption[];
  inputDeviceDefaultLabel: string;
  selectedInputDeviceId: string;
  deviceNotice: string;
  status: string;
  transcript: string;
  cleanup: CleanupSnapshot;
  cleanupApiKeyConfigured: boolean;
  cleanupCredentialBusy: boolean;
  cleanupActionPending: boolean;
  cleanupApiKeyStatus: string;
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
  selectedWindowShortcut = "",
): AppState => ({
  activeView: "capture",
  recording: "idle",
  shortcutCapture: "idle",
  selectedShortcut,
  selectedWindowShortcut,
  shortcutCaptureTarget: "recording",
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
  providerSelectionStatus: "loading",
  activeSessionId: null,
  providerApiKeyConfigured: { soniox: false, deepgram: false },
  providerApiKeyStatus: { soniox: "", deepgram: "" },
  inputDevices: [],
  inputDeviceDefaultLabel: "",
  selectedInputDeviceId,
  deviceNotice: "",
  status: "Ready.",
  transcript: "",
  cleanup: emptyCleanup(),
  cleanupApiKeyConfigured: false,
  cleanupCredentialBusy: false,
  cleanupActionPending: false,
  cleanupApiKeyStatus: "",
  partialTranscript: "",
  recordingStartedAt: null,
});

export const isRecording = (state: AppState) => state.recording === "recording";
export const isBusy = (state: AppState) =>
  state.recording === "starting" || state.recording === "stopping";

export const canChangeProvider = (state: AppState) =>
  !isBusy(state) && !isRecording(state);

export const canToggleRecording = (state: AppState) =>
  !isBusy(state) && (isRecording(state) || state.providerSelectionStatus === "ready");

export const isCapturingShortcut = (state: AppState) =>
  state.shortcutCapture === "capturing";

export const startShortcutCapture = (
  state: AppState,
  shortcutCaptureTarget: "recording" | "window" = "recording",
): AppState => ({
  ...state,
  shortcutCapture: "capturing",
  shortcutCaptureTarget,
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
    return snapshot.sessionId === state.activeSessionId
      ? state.recordingStartedAt ?? Date.now()
      : Date.now();
  }

  return null;
};

const recordingPhase: Record<RecordingState, number> = {
  idle: 0,
  starting: 1,
  recording: 2,
  stopping: 3,
  transcribed: 4,
  error: 4,
};

export const applyBackendSnapshot = (
  state: AppState,
  snapshot: BackendAppSnapshot,
): AppState => {
  // Command replies and events may arrive in a different order. Sessions are
  // monotonically numbered by the resident backend; never rewind a capture.
  if (state.activeSessionId !== null) {
    if (
      snapshot.sessionId === null ||
      snapshot.sessionId < state.activeSessionId ||
      (snapshot.sessionId === state.activeSessionId &&
        (recordingPhase[snapshot.status] < recordingPhase[state.recording] ||
          (state.recording === "error" && snapshot.status === "transcribed")))
    ) {
      return state;
    }
  }

  if (snapshot.sessionId === state.activeSessionId &&
      (snapshot.cleanup?.revision ?? 0) < state.cleanup.revision) {
    return state;
  }

  const isMissingApiKey = snapshot.error?.type === "missing_api_key";
  const sessionChanged = snapshot.sessionId !== state.activeSessionId;
  const shouldClearTranscript =
    sessionChanged || snapshot.status === "starting" || snapshot.status === "error";
  const shouldShowCapture =
    snapshot.status !== "idle" && snapshot.status !== "error" &&
    (sessionChanged || snapshot.status !== state.recording);

  return {
    ...state,
    activeView: isMissingApiKey
      ? "settings"
      : shouldShowCapture
        ? "capture"
        : state.activeView,
    providerSelectionStatus: isMissingApiKey ? "ready" : state.providerSelectionStatus,
    activeProvider: isMissingApiKey
      ? snapshot.error?.provider ?? state.activeProvider
      : state.activeProvider,
    recording: snapshot.status,
    activeSessionId: snapshot.sessionId,
    status: statusText(snapshot),
    cleanup: snapshot.cleanup ?? emptyCleanup(),
    cleanupActionPending: sessionChanged ? false : state.cleanupActionPending,
    transcript:
      snapshot.transcript?.text ??
      (shouldClearTranscript ? "" : state.transcript),
    partialTranscript:
      snapshot.status === "recording" && !sessionChanged ? state.partialTranscript : "",
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

export const saveWindowShortcut = (
  state: AppState,
  selectedWindowShortcut: string,
): AppState => ({
  ...state,
  shortcutCapture: "idle",
  selectedWindowShortcut,
  status: "Window shortcut saved.",
});

export const setActiveProvider = (
  state: AppState,
  activeProvider: ProviderId,
): AppState => ({
  ...state,
  activeProvider,
  providerSelectionStatus: "ready",
});

export const setProviderSelectionStatus = (
  state: AppState,
  providerSelectionStatus: ProviderSelectionStatus,
): AppState => ({ ...state, providerSelectionStatus });

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

export const canCleanTranscript = (state: AppState): boolean =>
  state.cleanupApiKeyConfigured && Boolean(state.transcript.trim()) &&
  canChangeProvider(state) && !state.cleanup.running && !state.cleanupActionPending && !state.cleanupCredentialBusy;

export const setCleanupCredentials = (
  state: AppState, configured: boolean, message = "",
): AppState => ({ ...state, cleanupApiKeyConfigured: configured, cleanupApiKeyStatus: message });

export const setCleanupCredentialStatus = (state: AppState, message: string): AppState =>
  ({ ...state, cleanupApiKeyStatus: message });

export const shouldAutoCopyTranscript = (state: AppState, lastSessionId: number | null): boolean =>
  state.autoCopyTranscript && state.recording === "transcribed" && Boolean(state.transcript) &&
  state.activeSessionId !== null && state.activeSessionId !== lastSessionId &&
  state.cleanup.revision === 0;
