import "./styles.css";
import {
  cancelShortcutCapture,
  createAppState,
  isCapturingShortcut,
  isRecording,
  saveShortcut,
  setStatus,
  startShortcutCapture,
  toggleRecording,
} from "./app-state";
import { createAppView } from "./app-view";
import { formatShortcut } from "./shortcut";
import { getGlobalShortcut, saveGlobalShortcut } from "./settings";
import { onGlobalShortcutPressed, setGlobalShortcut } from "./tauri";

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

  view.keybindButton.textContent = isCapturingShortcut(state)
    ? "Press keys"
    : state.selectedShortcut || "Set shortcut";
  view.keybindStatus.textContent = state.status;
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

view.recordButton.addEventListener("click", () => {
  updateState(toggleRecording(state));
});

view.keybindButton.addEventListener("click", () => {
  updateState(startShortcutCapture(state));
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
  updateState(toggleRecording(state));
}).catch(() => {
  updateState(
    setStatus(state, "Shortcut registration runs in the desktop app."),
  );
});

render();

if (state.selectedShortcut) {
  void registerShortcut(state.selectedShortcut);
}
