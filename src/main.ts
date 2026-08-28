import "./styles.css";
import {
  cancelShortcutCapture,
  createAppState,
  applyBackendSnapshot,
  applyPartialTranscript,
  isCapturingShortcut,
  isBusy,
  isRecording,
  saveShortcut,
  setApiKeyPresence,
  setApiKeyStatus,
  setAutoCopyTranscript,
  setLiveTranscript,
  setLaunchOnStartup,
  setLaunchOnStartupStatus,
  setMaxRecordingSeconds,
  setShowPartialTranscript,
  setShortcutFocusOnStart,
  setShortcutHideOnStop,
  setStatus,
  showCapture,
  showSettings,
  startShortcutCapture,
} from "./app-state";
import { createAppView } from "./app-view";
import { formatShortcut } from "./shortcut";
import {
  getAutoCopyTranscript,
  getGlobalShortcut,
  getLiveTranscript,
  getLaunchOnStartup,
  getMaxRecordingSeconds,
  getShowPartialTranscript,
  getShortcutFocusOnStart,
  getShortcutHideOnStop,
  saveAutoCopyTranscript,
  saveGlobalShortcut,
  saveLiveTranscript,
  saveLaunchOnStartup,
  saveMaxRecordingSeconds,
  saveShowPartialTranscript,
  saveShortcutFocusOnStart,
  saveShortcutHideOnStop,
} from "./settings";
import {
  copyTextToClipboard,
  deleteSonioxApiKey,
  deleteLocalLogs,
  enableTemporaryDebugLogging,
  exportLogs,
  getAppState,
  getLoggingStatus,
  getLaunchOnStartup as getBackendLaunchOnStartup,
  hasSonioxApiKey,
  onAppStateChanged,
  onPartialTranscript,
  saveSonioxApiKey,
  setGlobalShortcut,
  setLaunchOnStartup as setBackendLaunchOnStartup,
  setShortcutBehavior,
  toggleBackendRecording,
} from "./tauri";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root was not found");
}

let state = createAppState(
  getGlobalShortcut(),
  getMaxRecordingSeconds(),
  getAutoCopyTranscript(),
  getShortcutFocusOnStart(),
  getShortcutHideOnStop(),
  getLiveTranscript(),
  getShowPartialTranscript(),
  getLaunchOnStartup(),
);
const view = createAppView(app);
let lastAutoCopiedTranscript = "";
let loggingMessage = "";
let debugLoggingUntil = 0;

const formatElapsedTime = (startedAt: number | null): string => {
  if (!startedAt) {
    return "00:00";
  }

  const elapsedSeconds = Math.max(
    0,
    Math.floor((Date.now() - startedAt) / 1000),
  );
  const minutes = Math.floor(elapsedSeconds / 60).toString().padStart(2, "0");
  const seconds = (elapsedSeconds % 60).toString().padStart(2, "0");

  return `${minutes}:${seconds}`;
};

const statusLabel = () => {
  switch (state.recording) {
    case "idle":
      return "Ready";
    case "starting":
      return "Starting";
    case "recording":
      return "Listening";
    case "stopping":
      return "Finalizing";
    case "transcribed":
      return "Transcript ready";
    case "error":
      return "Error";
  }
};

const render = () => {
  const isSettings = state.activeView === "settings";
  view.captureView.hidden = isSettings;
  view.settingsView.hidden = !isSettings;
  view.viewToggleButton.textContent = isSettings ? "Capture" : "Settings";
  view.viewToggleButton.setAttribute(
    "aria-label",
    isSettings ? "Open capture" : "Open settings",
  );
  view.statusChip.textContent = statusLabel();
  view.statusChip.dataset.status = state.recording;

  view.recordButton.textContent = isRecording(state) ? "Stop" : "Record";
  view.recordButton.setAttribute(
    "aria-label",
    isRecording(state) ? "Stop recording" : "Start recording",
  );
  view.recordButton.disabled = isBusy(state);
  view.recordStatus.textContent = statusLabel();
  view.recordingTimer.textContent = formatElapsedTime(state.recordingStartedAt);
  view.activityIndicator.hidden = !isRecording(state);
  view.copyButton.disabled = !state.transcript;
  const hasTranscriptText =
    Boolean(state.transcript) || Boolean(state.partialTranscript);
  view.transcriptPlaceholder.hidden = hasTranscriptText;
  view.transcriptFinal.textContent = state.transcript;
  view.transcriptPartial.textContent = state.partialTranscript;
  view.transcriptText.classList.toggle("is-empty", !hasTranscriptText);

  view.keybindButton.textContent = isCapturingShortcut(state)
    ? "Press keys"
    : state.selectedShortcut || "Set shortcut";
  view.keybindStatus.textContent = state.status;
  view.shortcutFocusOnStartCheckbox.checked = state.shortcutFocusOnStart;
  view.shortcutHideOnStopCheckbox.checked = state.shortcutHideOnStop;
  view.apiKeyDeleteButton.disabled = !state.hasApiKey;
  view.apiKeyStatus.textContent =
    state.apiKeyStatus ||
    (state.hasApiKey ? "API key saved." : "No API key saved.");
  if (document.activeElement !== view.maxRecordingSecondsInput) {
    view.maxRecordingSecondsInput.value = state.maxRecordingSeconds.toString();
  }
  view.autoCopyCheckbox.checked = state.autoCopyTranscript;
  view.liveTranscriptCheckbox.checked = state.liveTranscript;
  view.showPartialField.hidden = !state.liveTranscript;
  view.showPartialCheckbox.checked = state.showPartialTranscript;
  view.launchOnStartupCheckbox.checked = state.launchOnStartup;
  view.launchOnStartupStatus.textContent =
    state.launchOnStartupStatus ||
    "Starts hidden in the tray when you sign in.";
  const debugSecondsRemaining = Math.max(
    0,
    Math.ceil((debugLoggingUntil - Date.now()) / 1000),
  );
  view.enableDebugLoggingButton.textContent = debugSecondsRemaining
    ? `Debug logging enabled (${Math.ceil(debugSecondsRemaining / 60)} min left)`
    : "Enable debug logging for 30 minutes";
  view.enableDebugLoggingButton.disabled = debugSecondsRemaining > 0;
  view.loggingStatus.textContent = loggingMessage;
  view.bottomStatus.textContent =
    state.activeView === "capture"
      ? state.status
      : state.selectedShortcut
        ? `Shortcut: ${state.selectedShortcut}`
        : "No shortcut saved.";
};

const updateState = (nextState: typeof state) => {
  state = nextState;
  render();
};

const copyTranscript = async (successMessage = "Transcript copied.") => {
  if (!state.transcript) {
    return;
  }

  try {
    await copyTextToClipboard(state.transcript);
    updateState(setStatus(state, successMessage));
  } catch (error) {
    updateState(
      setStatus(state, error instanceof Error ? error.message : String(error)),
    );
  }
};

const maybeAutoCopyTranscript = () => {
  if (
    !state.autoCopyTranscript ||
    state.recording !== "transcribed" ||
    !state.transcript ||
    state.transcript === lastAutoCopiedTranscript
  ) {
    return;
  }

  lastAutoCopiedTranscript = state.transcript;
  void copyTranscript("Transcript copied automatically.");
};

const registerShortcut = async (shortcut: string) => {
  try {
    await setGlobalShortcut(shortcut);
    saveGlobalShortcut(shortcut);
    updateState(saveShortcut(state, shortcut));
  } catch (error) {
    updateState(
      setStatus(state, error instanceof Error ? error.message : String(error)),
    );
  }
};

const pushShortcutBehavior = async () => {
  try {
    await setShortcutBehavior(
      state.shortcutFocusOnStart,
      state.shortcutHideOnStop,
    );
  } catch (error) {
    updateState(
      setStatus(state, error instanceof Error ? error.message : String(error)),
    );
  }
};

const loadApiKeyStatus = async () => {
  try {
    updateState(setApiKeyPresence(state, await hasSonioxApiKey()));
  } catch (error) {
    updateState(
      setApiKeyStatus(
        state,
        error instanceof Error ? error.message : String(error),
      ),
    );
  }
};

const loadBackendState = async () => {
  try {
    updateState(applyBackendSnapshot(state, await getAppState()));
  } catch (error) {
    updateState(
      setStatus(state, error instanceof Error ? error.message : String(error)),
    );
  }
};

const loadLaunchOnStartup = async () => {
  try {
    const enabled = await getBackendLaunchOnStartup();
    saveLaunchOnStartup(enabled);
    updateState(setLaunchOnStartup(state, enabled));
  } catch (error) {
    updateState(
      setLaunchOnStartupStatus(
        state,
        error instanceof Error ? error.message : String(error),
      ),
    );
  }
};

const updateLaunchOnStartup = async (enabled: boolean) => {
  view.launchOnStartupCheckbox.disabled = true;
  updateState(setLaunchOnStartupStatus(state, "Updating startup setting..."));

  try {
    const actualState = await setBackendLaunchOnStartup(enabled);
    saveLaunchOnStartup(actualState);
    updateState(
      setLaunchOnStartup(
        state,
        actualState,
        actualState
          ? "QuickText will start hidden in the tray when you sign in."
          : "QuickText will not launch when you sign in.",
      ),
    );
  } catch (error) {
    updateState(
      setLaunchOnStartupStatus(
        state,
        error instanceof Error ? error.message : String(error),
      ),
    );
  } finally {
    view.launchOnStartupCheckbox.disabled = false;
  }
};

const applyLoggingStatus = (debugSecondsRemaining: number) => {
  debugLoggingUntil = Date.now() + debugSecondsRemaining * 1000;
  render();
};

const loadLoggingStatus = async () => {
  try {
    const status = await getLoggingStatus();
    applyLoggingStatus(status.debugSecondsRemaining);
  } catch (error) {
    loggingMessage = error instanceof Error ? error.message : String(error);
    render();
  }
};

const enableDebugLogging = async () => {
  const confirmed = window.confirm(
    "Enable more detailed lifecycle and timing logs for 30 minutes? The same privacy exclusions continue to apply, and debug logging stops when QuickText exits.",
  );
  if (!confirmed) return;

  try {
    const status = await enableTemporaryDebugLogging();
    loggingMessage = "Temporary debug logging is enabled.";
    applyLoggingStatus(status.debugSecondsRemaining);
  } catch (error) {
    loggingMessage = error instanceof Error ? error.message : String(error);
    render();
  }
};

const exportSupportLogs = async () => {
  const confirmed = window.confirm(
    "Export local support logs? The ZIP includes build/platform details, timestamps, anonymous session IDs, lifecycle/timing data, audio format/counts, and stable error categories. It excludes audio, transcripts, API keys, clipboard contents, provider payloads, device names, network addresses, and identifying paths. QuickText will not upload it.",
  );
  if (!confirmed) return;

  try {
    if (await exportLogs()) {
      loggingMessage = "Support logs exported. You choose whether to share the ZIP.";
      render();
    }
  } catch (error) {
    loggingMessage = error instanceof Error ? error.message : String(error);
    render();
  }
};

const deleteSupportLogs = async () => {
  const confirmed = window.confirm(
    "Delete all logs retained by QuickText? Previously exported ZIP archives will not be deleted.",
  );
  if (!confirmed) return;

  try {
    await deleteLocalLogs();
    loggingMessage = "Local logs deleted. QuickText started a fresh log.";
  } catch (error) {
    loggingMessage = error instanceof Error ? error.message : String(error);
  }
  render();
};

const toggleRecording = async () => {
  try {
    const nextState = showCapture(state);
    state = nextState;
    updateState(
      applyBackendSnapshot(
        state,
        await toggleBackendRecording(state.maxRecordingSeconds),
      ),
    );
    maybeAutoCopyTranscript();
  } catch (error) {
    updateState(
      setStatus(state, error instanceof Error ? error.message : String(error)),
    );
  }
};

const saveApiKey = async () => {
  try {
    const hasApiKey = await saveSonioxApiKey(view.apiKeyInput.value);
    view.apiKeyInput.value = "";
    updateState(
      setApiKeyPresence(
        state,
        hasApiKey,
        hasApiKey
          ? "API key saved."
          : "The API key could not be read after saving.",
      ),
    );
  } catch (error) {
    updateState(
      setApiKeyStatus(
        state,
        error instanceof Error ? error.message : String(error),
      ),
    );
  }
};

const deleteApiKey = async () => {
  try {
    await deleteSonioxApiKey();
    view.apiKeyInput.value = "";
    updateState(setApiKeyPresence(state, false, "API key deleted."));
  } catch (error) {
    updateState(
      setApiKeyStatus(
        state,
        error instanceof Error ? error.message : String(error),
      ),
    );
  }
};

view.recordButton.addEventListener("click", () => {
  void toggleRecording();
});

view.viewToggleButton.addEventListener("click", () => {
  updateState(
    state.activeView === "settings" ? showCapture(state) : showSettings(state),
  );
});

view.copyButton.addEventListener("click", () => {
  void copyTranscript();
});

view.maxRecordingSecondsInput.addEventListener("change", () => {
  const seconds = Number(view.maxRecordingSecondsInput.value);

  if (!Number.isInteger(seconds) || seconds < 1) {
    view.maxRecordingSecondsInput.value = state.maxRecordingSeconds.toString();
    updateState(setStatus(state, "Enter a recording limit of at least 1 second."));
    return;
  }

  saveMaxRecordingSeconds(seconds);
  updateState(setMaxRecordingSeconds(state, seconds));
});

view.autoCopyCheckbox.addEventListener("change", () => {
  const autoCopyTranscript = view.autoCopyCheckbox.checked;
  saveAutoCopyTranscript(autoCopyTranscript);
  updateState(setAutoCopyTranscript(state, autoCopyTranscript));
});

view.liveTranscriptCheckbox.addEventListener("change", () => {
  const liveTranscript = view.liveTranscriptCheckbox.checked;
  saveLiveTranscript(liveTranscript);
  updateState(setLiveTranscript(state, liveTranscript));
});

view.showPartialCheckbox.addEventListener("change", () => {
  const showPartialTranscript = view.showPartialCheckbox.checked;
  saveShowPartialTranscript(showPartialTranscript);
  updateState(setShowPartialTranscript(state, showPartialTranscript));
});

view.launchOnStartupCheckbox.addEventListener("change", () => {
  void updateLaunchOnStartup(view.launchOnStartupCheckbox.checked);
});

view.enableDebugLoggingButton.addEventListener("click", () => {
  void enableDebugLogging();
});

view.exportLogsButton.addEventListener("click", () => {
  void exportSupportLogs();
});

view.deleteLogsButton.addEventListener("click", () => {
  void deleteSupportLogs();
});

view.keybindButton.addEventListener("click", () => {
  updateState(startShortcutCapture(state));
});

view.shortcutFocusOnStartCheckbox.addEventListener("change", () => {
  const focusOnStart = view.shortcutFocusOnStartCheckbox.checked;
  saveShortcutFocusOnStart(focusOnStart);
  updateState(setShortcutFocusOnStart(state, focusOnStart));
  void pushShortcutBehavior();
});

view.shortcutHideOnStopCheckbox.addEventListener("change", () => {
  const hideOnStop = view.shortcutHideOnStopCheckbox.checked;
  saveShortcutHideOnStop(hideOnStop);
  updateState(setShortcutHideOnStop(state, hideOnStop));
  void pushShortcutBehavior();
});

view.apiKeySaveButton.addEventListener("click", () => {
  void saveApiKey();
});

view.apiKeyDeleteButton.addEventListener("click", () => {
  void deleteApiKey();
});

window.addEventListener("keydown", (event) => {
  if (!isCapturingShortcut(state)) {
    return;
  }

  event.preventDefault();

  if (event.key === "Escape") {
    updateState(cancelShortcutCapture(state));
    return;
  }

  const result = formatShortcut(event);

  if (!result.ok) {
    if (result.message) {
      updateState(setStatus(state, result.message));
    }
    return;
  }

  void registerShortcut(result.shortcut);
});

void onAppStateChanged((snapshot) => {
  updateState(applyBackendSnapshot(state, snapshot));
  maybeAutoCopyTranscript();
}).catch(() => {
  updateState(
    setStatus(state, "Backend state events run in the desktop app."),
  );
});

void onPartialTranscript((update) => {
  updateState(applyPartialTranscript(state, update));
}).catch(() => {
  updateState(
    setStatus(state, "Live transcript events run in the desktop app."),
  );
});

render();
window.setInterval(render, 1000);
void loadBackendState();
void loadApiKeyStatus();
void loadLaunchOnStartup();
void loadLoggingStatus();
void pushShortcutBehavior();

if (state.selectedShortcut) {
  void registerShortcut(state.selectedShortcut);
}
