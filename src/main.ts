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
  setDeviceNotice,
  setInputDeviceOptions,
  setSelectedInputDevice,
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
  getInputDeviceId,
  getLiveTranscript,
  getLaunchOnStartup,
  getMaxRecordingSeconds,
  getShowPartialTranscript,
  getShortcutFocusOnStart,
  getShortcutHideOnStop,
  saveAutoCopyTranscript,
  saveGlobalShortcut,
  saveInputDeviceId,
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
  getAppState,
  getLaunchOnStartup as getBackendLaunchOnStartup,
  hasSonioxApiKey,
  listInputDevices,
  onAppStateChanged,
  onDeviceFallback,
  onPartialTranscript,
  saveSonioxApiKey,
  setGlobalShortcut,
  setInputDevice,
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
  getInputDeviceId(),
  getLaunchOnStartup(),
);
const view = createAppView(app);
let lastAutoCopiedTranscript = "";

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

let renderedDeviceSignature: string | null = null;

const renderInputDeviceSelect = () => {
  const signature = [
    state.inputDeviceDefaultLabel,
    ...state.inputDevices.map((device) => device.id),
  ].join("\u0000");

  if (signature !== renderedDeviceSignature) {
    view.inputDeviceSelect.innerHTML = "";
    const defaultOption = document.createElement("option");
    defaultOption.value = "";
    defaultOption.textContent = state.inputDeviceDefaultLabel
      ? `System default (${state.inputDeviceDefaultLabel})`
      : "System default";
    view.inputDeviceSelect.appendChild(defaultOption);

    for (const device of state.inputDevices) {
      const option = document.createElement("option");
      option.value = device.id;
      option.textContent = device.label;
      view.inputDeviceSelect.appendChild(option);
    }

    renderedDeviceSignature = signature;
  }

  const knownIds = ["", ...state.inputDevices.map((device) => device.id)];
  view.inputDeviceSelect.value = knownIds.includes(
    state.selectedInputDeviceId,
  )
    ? state.selectedInputDeviceId
    : "";
  const selectedKnown =
    !state.selectedInputDeviceId ||
    state.inputDevices.some(
      (device) => device.id === state.selectedInputDeviceId,
    );
  view.inputDeviceStatus.textContent = selectedKnown
    ? ""
    : `Saved microphone "${state.selectedInputDeviceId}" is currently unavailable.`;
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
  view.deviceNotice.textContent = state.deviceNotice;
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
  renderInputDeviceSelect();
  view.launchOnStartupCheckbox.checked = state.launchOnStartup;
  view.launchOnStartupStatus.textContent =
    state.launchOnStartupStatus ||
    "Starts hidden in the tray when you sign in.";
  view.bottomStatus.textContent =
    state.activeView === "capture"
      ? state.status
      : state.selectedShortcut
        ? `Shortcut: ${state.selectedShortcut}`
        : "No shortcut saved.";
};

const updateState = (nextState: typeof state) => {
  const wasSettings = state.activeView === "settings";
  state = nextState;
  render();

  if (!wasSettings && state.activeView === "settings") {
    void refreshInputDevices();
  }
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

const refreshInputDevices = async () => {
  try {
    const devices = await listInputDevices();
    updateState(
      setInputDeviceOptions(state, devices.devices, devices.defaultLabel ?? ""),
    );
  } catch (error) {
    updateState(
      setStatus(state, error instanceof Error ? error.message : String(error)),
    );
  }
};

const saveInputDeviceSelection = async (deviceId: string) => {
  try {
    await setInputDevice(deviceId || null);
    saveInputDeviceId(deviceId);
    updateState(setSelectedInputDevice(state, deviceId));
  } catch (error) {
    view.inputDeviceSelect.value = state.selectedInputDeviceId;
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

view.inputDeviceSelect.addEventListener("change", () => {
  void saveInputDeviceSelection(view.inputDeviceSelect.value);
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

void onDeviceFallback((message) => {
  updateState(setDeviceNotice(state, message));
}).catch(() => {
  // Device fallback events run in the desktop app only.
});

render();
window.setInterval(render, 1000);
void loadBackendState();
void loadApiKeyStatus();
void loadLaunchOnStartup();
void pushShortcutBehavior();

if (state.selectedInputDeviceId) {
  setInputDevice(state.selectedInputDeviceId).catch(() => {
    // The saved device may be missing; recording falls back to the default.
  });
}

if (state.selectedShortcut) {
  void registerShortcut(state.selectedShortcut);
}
