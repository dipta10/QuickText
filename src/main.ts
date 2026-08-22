import "./styles.css";
import {
  cancelShortcutCapture,
  createAppState,
  applyBackendSnapshot,
  isCapturingShortcut,
  isBusy,
  isRecording,
  saveShortcut,
  setApiKeyPresence,
  setApiKeyStatus,
  setStatus,
  startShortcutCapture,
} from "./app-state";
import { createAppView } from "./app-view";
import { formatShortcut } from "./shortcut";
import { getGlobalShortcut, saveGlobalShortcut } from "./settings";
import {
  deleteSonioxApiKey,
  getAppState,
  hasSonioxApiKey,
  onAppStateChanged,
  onGlobalShortcutPressed,
  saveSonioxApiKey,
  setGlobalShortcut,
  toggleBackendRecording,
} from "./tauri";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root was not found");
}

let state = createAppState(getGlobalShortcut());
const view = createAppView(app);

const render = () => {
  view.recordButton.textContent = isRecording(state) ? "Stop" : "Record";
  view.recordButton.setAttribute(
    "aria-label",
    isRecording(state) ? "Stop recording" : "Start recording",
  );
  view.recordButton.disabled = isBusy(state);
  view.transcriptOutput.value = state.transcript;

  view.keybindButton.textContent = isCapturingShortcut(state)
    ? "Press keys"
    : state.selectedShortcut || "Set shortcut";
  view.keybindStatus.textContent = state.status;
  view.apiKeyDeleteButton.disabled = !state.hasApiKey;
  view.apiKeyStatus.textContent =
    state.apiKeyStatus || (state.hasApiKey ? "API key saved." : "No API key saved.");
};

const updateState = (nextState: typeof state) => {
  state = nextState;
  render();
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

const toggleRecording = async () => {
  try {
    updateState(applyBackendSnapshot(state, await toggleBackendRecording()));
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

view.keybindButton.addEventListener("click", () => {
  updateState(startShortcutCapture(state));
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

void onGlobalShortcutPressed(() => {
  void toggleRecording();
}).catch(() => {
  updateState(
    setStatus(state, "Shortcut registration runs in the desktop app."),
  );
});

void onAppStateChanged((snapshot) => {
  updateState(applyBackendSnapshot(state, snapshot));
}).catch(() => {
  updateState(
    setStatus(state, "Backend state events run in the desktop app."),
  );
});

render();
void loadBackendState();
void loadApiKeyStatus();

if (state.selectedShortcut) {
  void registerShortcut(state.selectedShortcut);
}
