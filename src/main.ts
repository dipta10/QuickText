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
  setPasteToTarget,
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
  getPasteToTarget,
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
  savePasteToTarget,
} from "./settings";
import {
  copyTextToClipboard,
  deleteSonioxApiKey,
  getAppState,
  getBuildInfo,
  getLaunchOnStartup as getBackendLaunchOnStartup,
  getLanguagePreferences,
  getTranscriptionDescription,
  hasSonioxApiKey,
  listInputDevices,
  onAppStateChanged,
  onDeviceFallback,
  onPartialTranscript,
  saveSonioxApiKey,
  setGlobalShortcut,
  setInputDevice,
  setPasteToTargetBackend,
  setLaunchOnStartup as setBackendLaunchOnStartup,
  setLanguagePreferences,
  setTranscriptionDescription,
  setShortcutBehavior,
  toggleBackendRecording,
} from "./tauri";
import type { SupportedLanguage } from "./tauri";

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
  getPasteToTarget(),
  getLaunchOnStartup(),
);
const view = createAppView(app);
let lastAutoCopiedTranscript = "";
let availableLanguages: SupportedLanguage[] = [];
let selectedLanguageCodes: string[] = [];
let confirmedLanguageCodes: string[] = [];
let languageSearch = "";
let languagePreferencesSaving = false;
let transcriptionDescriptionSaving = false;

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

const languageSummary = () => {
  if (selectedLanguageCodes.length === 0) {
    return "Automatic detection";
  }

  if (selectedLanguageCodes.length === 1) {
    return (
      availableLanguages.find(
        (language) => language.code === selectedLanguageCodes[0],
      )?.name ?? selectedLanguageCodes[0]
    );
  }

  return `${selectedLanguageCodes.length} languages selected`;
};

const renderLanguagePicker = () => {
  view.languagePickerSummary.textContent = languageSummary();
  view.clearLanguagesButton.disabled =
    languagePreferencesSaving || selectedLanguageCodes.length === 0;
  view.languageSearchInput.disabled = languagePreferencesSaving;
  view.languageOptions.innerHTML = "";

  const query = languageSearch.trim().toLocaleLowerCase();
  const visibleLanguages = availableLanguages.filter(
    (language) =>
      !query ||
      language.name.toLocaleLowerCase().includes(query) ||
      language.code.includes(query),
  );

  for (const language of visibleLanguages) {
    const label = document.createElement("label");
    label.className = "language-option";
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.value = language.code;
    checkbox.checked = selectedLanguageCodes.includes(language.code);
    checkbox.disabled = languagePreferencesSaving;
    const name = document.createElement("span");
    name.textContent = language.name;
    const code = document.createElement("span");
    code.className = "language-code";
    code.textContent = language.code;
    label.append(checkbox, name, code);
    view.languageOptions.appendChild(label);
  }

  if (visibleLanguages.length === 0) {
    const empty = document.createElement("p");
    empty.className = "settings-note";
    empty.textContent = "No languages match your search.";
    view.languageOptions.appendChild(empty);
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
  view.pasteToTargetCheckbox.checked = state.pasteToTarget;
  view.pasteToTargetNote.hidden = !state.pasteToTarget;
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

const loadLanguagePreferences = async () => {
  try {
    const snapshot = await getLanguagePreferences();
    availableLanguages = snapshot.availableLanguages;
    selectedLanguageCodes = [...snapshot.selectedCodes];
    confirmedLanguageCodes = [...snapshot.selectedCodes];
    renderLanguagePicker();
  } catch (error) {
    view.languageStatus.textContent =
      error instanceof Error ? error.message : String(error);
  }
};

const saveSelectedLanguages = async (selectedCodes: string[]) => {
  if (languagePreferencesSaving) {
    return;
  }

  const previousCodes = [...confirmedLanguageCodes];
  selectedLanguageCodes = selectedCodes;
  languagePreferencesSaving = true;
  view.languageStatus.textContent = "Saving language preferences...";
  renderLanguagePicker();

  try {
    const savedCodes = await setLanguagePreferences(selectedCodes);
    selectedLanguageCodes = [...savedCodes];
    confirmedLanguageCodes = [...savedCodes];
    view.languageStatus.textContent = savedCodes.length
      ? "Language preferences saved. They apply to the next recording."
      : "Automatic language detection is enabled.";
  } catch (error) {
    selectedLanguageCodes = previousCodes;
    view.languageStatus.textContent =
      error instanceof Error ? error.message : String(error);
  } finally {
    languagePreferencesSaving = false;
    renderLanguagePicker();
  }
};

const loadTranscriptionDescription = async () => {
  try {
    view.transcriptionDescriptionInput.value =
      await getTranscriptionDescription();
  } catch (error) {
    view.transcriptionDescriptionStatus.textContent =
      error instanceof Error ? error.message : String(error);
  }
};

const saveTranscriptionDescription = async () => {
  if (transcriptionDescriptionSaving) {
    return;
  }

  transcriptionDescriptionSaving = true;
  view.transcriptionDescriptionSaveButton.disabled = true;
  view.transcriptionDescriptionStatus.textContent = "Saving description...";

  try {
    const description = await setTranscriptionDescription(
      view.transcriptionDescriptionInput.value,
    );
    view.transcriptionDescriptionInput.value = description;
    view.transcriptionDescriptionStatus.textContent = description.length
      ? "Description saved. It applies to the next recording."
      : "Description cleared. New recordings will send no context.";
  } catch (error) {
    view.transcriptionDescriptionStatus.textContent =
      error instanceof Error ? error.message : String(error);
  } finally {
    transcriptionDescriptionSaving = false;
    view.transcriptionDescriptionSaveButton.disabled = false;
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

const loadBuildInfo = async () => {
  try {
    const buildInfo = await getBuildInfo();
    view.appVersion.textContent = buildInfo.version;
    view.buildId.textContent = buildInfo.buildId;
    view.sourceRevision.textContent =
      buildInfo.sourceRevision === "Development"
        ? buildInfo.sourceRevision
        : buildInfo.sourceRevision.slice(0, 7);
  } catch {
    view.appVersion.textContent = "Unavailable";
    view.buildId.textContent = "Unavailable";
    view.sourceRevision.textContent = "Unavailable";
  }
};

const copyBuildInfo = async () => {
  const version = view.appVersion.textContent || "Unavailable";
  const buildId = view.buildId.textContent || "Unavailable";
  const sourceRevision = view.sourceRevision.textContent || "Unavailable";
  const text = `QuickText\nVersion: ${version}\nBuild: ${buildId}\nCommit: ${sourceRevision}`;

  try {
    await copyTextToClipboard(text);
    view.buildInfoStatus.textContent = "Build information copied.";
  } catch (error) {
    view.buildInfoStatus.textContent =
      error instanceof Error ? error.message : String(error);
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

view.copyBuildInfoButton.addEventListener("click", () => {
  void copyBuildInfo();
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

const pushPasteToTarget = async (pasteToTarget: boolean) => {
  try {
    await setPasteToTargetBackend(pasteToTarget);
  } catch (error) {
    updateState(
      setStatus(state, error instanceof Error ? error.message : String(error)),
    );
  }
};

view.pasteToTargetCheckbox.addEventListener("change", () => {
  const pasteToTarget = view.pasteToTargetCheckbox.checked;
  savePasteToTarget(pasteToTarget);
  updateState(setPasteToTarget(state, pasteToTarget));
  void pushPasteToTarget(pasteToTarget);
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

view.transcriptionDescriptionSaveButton.addEventListener("click", () => {
  void saveTranscriptionDescription();
});

view.inputDeviceSelect.addEventListener("change", () => {
  void saveInputDeviceSelection(view.inputDeviceSelect.value);
});

view.languageSearchInput.addEventListener("input", () => {
  languageSearch = view.languageSearchInput.value;
  renderLanguagePicker();
});

view.languageOptions.addEventListener("change", (event) => {
  const checkbox = event.target;
  if (!(checkbox instanceof HTMLInputElement) || checkbox.type !== "checkbox") {
    return;
  }

  const selected = new Set(selectedLanguageCodes);
  if (checkbox.checked) {
    selected.add(checkbox.value);
  } else {
    selected.delete(checkbox.value);
  }
  void saveSelectedLanguages(
    availableLanguages
      .map((language) => language.code)
      .filter((code) => selected.has(code)),
  );
});

view.clearLanguagesButton.addEventListener("click", () => {
  void saveSelectedLanguages([]);
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
void loadBuildInfo();
void loadApiKeyStatus();
void loadLanguagePreferences();
void loadTranscriptionDescription();
void loadLaunchOnStartup();
void pushShortcutBehavior();
void pushPasteToTarget(state.pasteToTarget);

if (state.selectedInputDeviceId) {
  setInputDevice(state.selectedInputDeviceId).catch(() => {
    // The saved device may be missing; recording falls back to the default.
  });
}

if (state.selectedShortcut) {
  void registerShortcut(state.selectedShortcut);
}
