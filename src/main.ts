import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("App root was not found");
}

const shortcutStorageKey = "stt.globalShortcut";
let isCapturingShortcut = false;
let isRecording = false;
let selectedShortcut = localStorage.getItem(shortcutStorageKey) ?? "";

app.innerHTML = `
  <section class="app-shell" aria-label="Speech to Text">
    <button class="record-button" type="button" aria-label="Start recording">
      Record
    </button>

    <div class="keybind-panel">
      <span class="keybind-label">Keybind</span>
      <button class="keybind-button" type="button">
        Set shortcut
      </button>
      <p class="keybind-status" role="status"></p>
    </div>
  </section>
`;

const recordButton =
  document.querySelector<HTMLButtonElement>(".record-button");
const keybindButton =
  document.querySelector<HTMLButtonElement>(".keybind-button");
const keybindStatus =
  document.querySelector<HTMLParagraphElement>(".keybind-status");

if (!recordButton || !keybindButton || !keybindStatus) {
  throw new Error("App controls were not found");
}

keybindButton.textContent = selectedShortcut || "Set shortcut";

const setStatus = (message: string) => {
  keybindStatus.textContent = message;
};

const setRecording = (recording: boolean) => {
  isRecording = recording;
  recordButton.textContent = isRecording ? "Stop" : "Record";
  recordButton.setAttribute(
    "aria-label",
    isRecording ? "Stop recording" : "Start recording",
  );
};

const registerShortcut = async (shortcut: string) => {
  try {
    await invoke("set_global_shortcut", { shortcut });
    localStorage.setItem(shortcutStorageKey, shortcut);
    selectedShortcut = shortcut;
    keybindButton.textContent = shortcut;
    setStatus("Saved.");
  } catch (error) {
    setStatus(error instanceof Error ? error.message : String(error));
  }
};

const formatKey = (event: KeyboardEvent): string | null => {
  if (/^Key[A-Z]$/.test(event.code)) {
    return event.code.replace("Key", "");
  }

  if (/^Digit[0-9]$/.test(event.code)) {
    return event.code.replace("Digit", "");
  }

  if (/^F\d{1,2}$/.test(event.code)) {
    return event.code;
  }

  const namedKeys: Record<string, string> = {
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
    ArrowUp: "ArrowUp",
    Backspace: "Backspace",
    Delete: "Delete",
    Enter: "Enter",
    Escape: "Escape",
    Space: "Space",
    Tab: "Tab",
  };

  return namedKeys[event.code] ?? null;
};

const formatShortcut = (event: KeyboardEvent): string | null => {
  const key = formatKey(event);

  if (!key || ["Control", "Shift", "Alt", "Meta"].includes(event.key)) {
    return null;
  }

  if (key === "Escape") {
    return "Escape";
  }

  const modifiers: string[] = [];

  if (event.ctrlKey || event.metaKey) {
    modifiers.push("CmdOrCtrl");
  }

  if (event.altKey) {
    modifiers.push("Alt");
  }

  if (event.shiftKey) {
    modifiers.push("Shift");
  }

  if (modifiers.length === 0) {
    setStatus("Use at least one modifier.");
    return null;
  }

  return [...modifiers, key].join("+");
};

recordButton.addEventListener("click", () => {
  setRecording(!isRecording);
});

keybindButton.addEventListener("click", () => {
  isCapturingShortcut = true;
  keybindButton.textContent = "Press keys";
  setStatus("Press a modifier plus a key. Esc cancels.");
});

window.addEventListener("keydown", (event) => {
  if (!isCapturingShortcut) {
    return;
  }

  event.preventDefault();

  if (event.key === "Escape") {
    isCapturingShortcut = false;
    keybindButton.textContent = selectedShortcut || "Set shortcut";
    setStatus("Canceled.");
    return;
  }

  const shortcut = formatShortcut(event);

  if (!shortcut) {
    return;
  }

  isCapturingShortcut = false;
  void registerShortcut(shortcut);
});

void listen<string>("global-shortcut-pressed", () => {
  setRecording(!isRecording);
}).catch(() => {
  setStatus("Shortcut registration runs in the desktop app.");
});

if (selectedShortcut) {
  void registerShortcut(selectedShortcut);
}
