// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BackendAppSnapshot, ProviderId } from "./app-state";
import * as backend from "./tauri";

vi.mock("./tauri");

const idle: BackendAppSnapshot = {
  status: "idle", sessionId: null, transcript: null, error: null,
};

const deferred = <T>() => {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((complete) => { resolve = complete; });
  return { promise, resolve };
};

const element = <T extends Element>(selector: string): T => {
  const result = document.querySelector<T>(selector);
  if (!result) throw new Error(`Missing ${selector}`);
  return result;
};

const expectProvider = (provider: ProviderId) => {
  expect(element<HTMLSelectElement>(".provider-select").value).toBe(provider);
  expect(element(".provider-label").textContent).toBe(
    `Provider: ${provider === "soniox" ? "Soniox" : "Deepgram"}`,
  );
};

const startApp = async (provider: ProviderId = "deepgram") => {
  const settings = deferred<backend.TranscriptionSettingsSnapshot>();
  vi.mocked(backend.getTranscriptionSettings).mockReturnValue(settings.promise);
  await import("./main");
  settings.resolve({ activeProvider: provider });
  await settings.promise;
};

// Reinitialize the frontend while preserving saved preferences, as on reopening.
const reopenApp = async () => {
  for (const [type, listener, options] of vi.mocked(window.addEventListener).mock.calls) {
    window.removeEventListener(type, listener, options);
  }
  vi.mocked(window.addEventListener).mockClear();
  vi.clearAllTimers();
  vi.resetModules();
  vi.mocked(backend.setGlobalShortcut).mockClear();
  vi.mocked(backend.setWindowShortcut).mockClear();
  document.body.innerHTML = '<div id="app"></div>';
  await import("./main");
  await Promise.resolve();
};

beforeEach(() => {
  vi.resetModules();
  vi.resetAllMocks();
  vi.useFakeTimers();
  vi.spyOn(window, "addEventListener");
  localStorage.clear();
  document.body.innerHTML = '<div id="app"></div>';
  vi.mocked(backend.getAppState).mockResolvedValue(idle);
  vi.mocked(backend.initializeShortcuts).mockResolvedValue({ recording: "", window: "", recordingError: null, windowError: null });
  vi.mocked(backend.getTranscriptionSettings).mockResolvedValue({ activeProvider: "deepgram" });
  vi.mocked(backend.hasProviderApiKey).mockResolvedValue(true);
  vi.mocked(backend.hasCleanupApiKey).mockResolvedValue(false);
  vi.mocked(backend.getBuildInfo).mockResolvedValue({
    version: "0.1.0", buildId: "test", sourceRevision: "Development",
  });
  vi.mocked(backend.getLaunchOnStartup).mockResolvedValue(false);
  vi.mocked(backend.getLanguagePreferences).mockResolvedValue({
    availableLanguages: [], selectedCodes: [],
  });
  vi.mocked(backend.getTranscriptionDescription).mockResolvedValue("");
  vi.mocked(backend.getTranscriptionTerms).mockResolvedValue("");
  vi.mocked(backend.listInputDevices).mockResolvedValue({ defaultLabel: null, devices: [] });
  vi.mocked(backend.onAppStateChanged).mockResolvedValue(() => {});
  vi.mocked(backend.onPartialTranscript).mockResolvedValue(() => {});
  vi.mocked(backend.onDeviceFallback).mockResolvedValue(() => {});
  vi.mocked(backend.onAudioLevel).mockResolvedValue(() => {});
});

afterEach(() => {
  for (const [type, listener, options] of vi.mocked(window.addEventListener).mock.calls) {
    window.removeEventListener(type, listener, options);
  }
  vi.restoreAllMocks();
  vi.clearAllTimers();
  vi.useRealTimers();
  document.body.innerHTML = "";
});

describe("provider restoration in the app", () => {
  it("restores the saved provider when all startup requests resolve immediately", async () => {
    await import("./main");
    expectProvider("deepgram");
    expect(element<HTMLButtonElement>(".record-button").disabled).toBe(false);
  });

  it.each([
    ["soniox", "soniox"], ["soniox", "deepgram"],
    ["deepgram", "soniox"], ["deepgram", "deepgram"],
  ] as const)(
    "keeps the saved %s provider when delayed credential checks finish with %s first",
    async (provider, firstProvider) => {
      const keys = { soniox: deferred<boolean>(), deepgram: deferred<boolean>() };
      vi.mocked(backend.hasProviderApiKey).mockImplementation(
        (id) => keys[id].promise,
      );
      await startApp(provider);
      expectProvider(provider);

      const secondProvider = firstProvider === "soniox" ? "deepgram" : "soniox";
      for (const id of [firstProvider, secondProvider] as const) {
        keys[id].resolve(true);
        await keys[id].promise;
        expectProvider(provider);
      }
      expect(element(".api-key-status").textContent).toBe("API key saved.");
      expect(element(".deepgram-api-key-status").textContent).toBe("API key saved.");
      expect(element<HTMLButtonElement>(".record-button").disabled).toBe(false);
    },
  );

  it("keeps the saved provider and newer recording event when the initial snapshot arrives late", async () => {
    const initialSnapshot = deferred<BackendAppSnapshot>();
    vi.mocked(backend.getAppState).mockReturnValue(initialSnapshot.promise);
    await startApp();
    expectProvider("deepgram");
    const onState = vi.mocked(backend.onAppStateChanged).mock.calls[0][0];
    onState({ ...idle, status: "recording", sessionId: 1 });

    initialSnapshot.resolve(idle);
    await initialSnapshot.promise;
    expectProvider("deepgram");
    expect(element(".status-chip").textContent).toBe("Listening");
    expect(element<HTMLButtonElement>(".record-button").textContent).toBe("Stop recording");
  });

  it("keeps the provider and finalized transcript when an older toggle reply arrives late", async () => {
    const toggleReply = deferred<BackendAppSnapshot>();
    vi.mocked(backend.toggleBackendRecording).mockReturnValue(toggleReply.promise);
    await startApp();
    element<HTMLButtonElement>(".record-button").click();
    expect(backend.toggleBackendRecording).toHaveBeenCalledOnce();
    const onState = vi.mocked(backend.onAppStateChanged).mock.calls[0][0];
    onState({
      ...idle, status: "transcribed", sessionId: 1,
      transcript: { text: "Final transcript", provider: "deepgram" },
    });

    toggleReply.resolve({ ...idle, status: "recording", sessionId: 1 });
    await toggleReply.promise;
    expectProvider("deepgram");
    expect(element(".status-chip").textContent).toBe("Transcript ready");
    expect(element(".transcript-final").textContent).toBe("Final transcript");
  });
});

it("shows actual microphone levels and ignores stopped or obsolete recording updates", async () => {
  await startApp("soniox");
  expect(backend.onAudioLevel).toHaveBeenCalledOnce();
  const onLevel = vi.mocked(backend.onAudioLevel).mock.calls[0][0];
  const onState = vi.mocked(backend.onAppStateChanged).mock.calls[0][0];
  const meter = element<HTMLElement>(".activity-bars");
  onState({ ...idle, status: "recording", sessionId: 1 });
  onLevel({ recordingSessionId: 1, level: 0 });
  expect(meter.getAttribute("aria-valuenow")).toBe("0");
  onLevel({ recordingSessionId: 1, level: 0.1 });
  const quietHeight = (meter.lastElementChild as HTMLElement).style.height;
  onLevel({ recordingSessionId: 1, level: 0.8 });
  expect(parseFloat((meter.lastElementChild as HTMLElement).style.height)).toBeGreaterThan(parseFloat(quietHeight));
  onLevel({ recordingSessionId: 2, level: 0 });
  expect(Number(meter.getAttribute("aria-valuenow"))).toBeGreaterThan(0);
  for (let index = 0; index < meter.children.length; index += 1) {
    onLevel({ recordingSessionId: 1, level: 0 });
  }
  expect(meter.getAttribute("aria-valuenow")).toBe("0");
  expect(Array.from(meter.children).every((bar) => (bar as HTMLElement).style.height === "3px")).toBe(true);
  onLevel({ recordingSessionId: 1, level: 0.5 });
  onState({ ...idle, status: "stopping", sessionId: 1 });
  onLevel({ recordingSessionId: 1, level: 1 });
  expect(meter.getAttribute("aria-valuenow")).toBe("0");
  expect(Array.from(meter.children).every((bar) => (bar as HTMLElement).style.height === "3px")).toBe(true);
  onState({ ...idle, status: "recording", sessionId: 2 });
  onLevel({ recordingSessionId: 1, level: 1 });
  expect(meter.getAttribute("aria-valuenow")).toBe("0");
  onLevel({ recordingSessionId: 2, level: 0.2 });
  expect(Number(meter.getAttribute("aria-valuenow"))).toBeGreaterThan(0);
});

it("keeps recording visuals and actions aligned with backend lifecycle events", async () => {
  await startApp("soniox");
  const onState = vi.mocked(backend.onAppStateChanged).mock.calls[0][0];
  const button = element<HTMLButtonElement>(".record-button");
  for (const [status, label, disabled] of [
    ["starting", "Starting…", true],
    ["recording", "Stop recording", false],
    ["stopping", "Finalizing…", true],
    ["transcribed", "Start recording", false],
  ] as const) {
    onState({ ...idle, status, sessionId: 1 });
    expect(element<HTMLElement>(".capture-view").dataset.status).toBe(status);
    expect(button.textContent).toBe(label);
    expect(button.getAttribute("aria-label")).toBe(label);
    expect(button.disabled).toBe(disabled);
    const statusLabel = element<HTMLElement>(".status-chip");
    expect(statusLabel.dataset.status).toBe(status);
    expect(statusLabel.closest(".recording-controls")).not.toBeNull();
    expect(element(".top-bar").querySelector(".status-chip")).toBeNull();
  }
});

describe("cleanup in the running app", () => {
  it("shows restore after recording, stopping and cleaning a short transcript, then restores it", async () => {
    vi.mocked(backend.hasCleanupApiKey).mockResolvedValue(true);
    await startApp("soniox");
    const onState = vi.mocked(backend.onAppStateChanged).mock.calls[0][0];
    const recorded: BackendAppSnapshot = {
      ...idle, status: "recording", sessionId: 1,
    };
    const original: BackendAppSnapshot = {
      ...recorded, status: "transcribed",
      transcript: { text: "Um, move it to Friday.", provider: "soniox" },
      cleanup: { revision: 0, running: false, cleaned: false, message: "" },
    };
    vi.mocked(backend.toggleBackendRecording).mockImplementationOnce(async () => {
      onState(recorded);
      return recorded;
    }).mockImplementationOnce(async () => {
      onState({ ...recorded, status: "stopping" });
      onState(original);
      return original;
    });
    element<HTMLButtonElement>(".record-button").click();
    await vi.advanceTimersByTimeAsync(0);
    element<HTMLButtonElement>(".record-button").click();
    await vi.advanceTimersByTimeAsync(0);
    expect(element<HTMLButtonElement>(".restore-button").hidden).toBe(true);
    expect(element<HTMLElement>(".cleanup-feedback").hidden).toBe(true);
    const cleaned: BackendAppSnapshot = {
      ...original, transcript: { text: "Move it to Friday.", provider: "soniox" },
      cleanup: { revision: 2, running: false, cleaned: true, message: "Cleaned and copied." },
    };
    vi.mocked(backend.cleanAndCopy).mockImplementation(async () => {
      onState({ ...original, cleanup: { revision: 1, running: true, cleaned: false, message: "Cleaning…" } });
      onState(cleaned);
    });
    element<HTMLButtonElement>(".clean-button").click();
    await vi.advanceTimersByTimeAsync(1000);
    expect(backend.cleanAndCopy).toHaveBeenCalledOnce();
    expect(element(".transcript-final").textContent).toBe("Move it to Friday.");
    expect(element<HTMLButtonElement>(".restore-button").hidden).toBe(false);
    expect(element<HTMLElement>(".cleanup-feedback").hidden).toBe(false);
    expect(element<HTMLElement>(".cleanup-status").hidden).toBe(false);
    expect(element(".cleanup-status").textContent).toBe("Cleaned and copied.");
    vi.mocked(backend.restoreOriginalTranscript).mockImplementation(async () => {
      onState({ ...original, cleanup: { revision: 3, running: false, cleaned: false, message: "Original restored. Press Copy to copy it." } });
    });
    element<HTMLButtonElement>(".restore-button").click();
    await vi.advanceTimersByTimeAsync(0);
    expect(backend.restoreOriginalTranscript).toHaveBeenCalledOnce();
    expect(element(".transcript-final").textContent).toBe(original.transcript?.text);
    expect(element<HTMLButtonElement>(".restore-button").hidden).toBe(true);
    expect(backend.copyTextToClipboard).not.toHaveBeenCalled();
  });
});


it("auto-copies each recording once, excludes cleanup and restore, and manually copies the displayed variant", async () => {
  localStorage.setItem("stt.autoCopyTranscript", "true");
  await startApp("soniox");
  const onState = vi.mocked(backend.onAppStateChanged).mock.calls[0][0];
  const original: BackendAppSnapshot = {
    status: "transcribed", sessionId: 1, error: null,
    transcript: { text: "Um, Friday.", provider: "soniox" },
  };
  onState(original);
  onState(original);
  await vi.advanceTimersByTimeAsync(0);
  expect(backend.copyTextToClipboard).toHaveBeenCalledExactlyOnceWith("Um, Friday.");
  onState({ ...original, transcript: { text: "Friday.", provider: "soniox" },
    cleanup: { revision: 2, running: false, cleaned: true, message: "Cleaned and copied." } });
  await vi.advanceTimersByTimeAsync(0);
  expect(backend.copyTextToClipboard).toHaveBeenCalledTimes(1);
  element<HTMLButtonElement>(".copy-button").click();
  await vi.advanceTimersByTimeAsync(0);
  expect(backend.copyTextToClipboard).toHaveBeenLastCalledWith("Friday.");
  onState({ ...original, cleanup: { revision: 3, running: false, cleaned: false, message: "Original restored." } });
  await vi.advanceTimersByTimeAsync(0);
  expect(backend.copyTextToClipboard).toHaveBeenCalledTimes(2);
  element<HTMLButtonElement>(".copy-button").click();
  await vi.advanceTimersByTimeAsync(0);
  expect(backend.copyTextToClipboard).toHaveBeenLastCalledWith("Um, Friday.");
  onState({ ...original, sessionId: 2 });
  await vi.advanceTimersByTimeAsync(0);
  expect(backend.copyTextToClipboard).toHaveBeenCalledTimes(4);
});

describe("native shortcut settings", () => {
  const saved: backend.ShortcutSettingsSnapshot = {
    recording: "F9", window: "F10", recordingError: null, windowError: null,
  };
  const controls = (target: "recording" | "window") => ({
    register: target === "window" ? backend.setWindowShortcut : backend.setGlobalShortcut,
    key: target === "window" ? ".window-keybind-button" : ".keybind-button:not(.window-keybind-button)",
    clear: target === "window" ? ".clear-window-keybind-button" : ".clear-keybind-button",
    status: target === "window" ? ".window-keybind-status" : ".keybind-status",
  });

  it("uses native assignments even when webview storage is empty or stale", async () => {
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    expect(element(".window-keybind-button").textContent).toBe("F10");
    expect(backend.initializeShortcuts).toHaveBeenCalledWith("", "");
    localStorage.setItem("stt.windowShortcut", "F12");
    await reopenApp();
    expect(backend.initializeShortcuts).toHaveBeenLastCalledWith("", "F12");
    expect(element(".window-keybind-button").textContent).toBe("F10");
    // The backend restores registration; reading Settings must not re-register.
    expect(backend.setGlobalShortcut).not.toHaveBeenCalled();
    expect(backend.setWindowShortcut).not.toHaveBeenCalled();
  });

  it("passes old local assignments to the backend for one-time migration", async () => {
    localStorage.setItem("stt.globalShortcut", "F9");
    localStorage.setItem("stt.windowShortcut", "F10");
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    expect(backend.initializeShortcuts).toHaveBeenCalledWith("F9", "F10");
  });

  it("loads native shortcuts when legacy shortcut storage cannot be read", async () => {
    const read = Storage.prototype.getItem;
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(function (this: Storage, key: string) {
      if (key === "stt.globalShortcut" || key === "stt.windowShortcut") {
        throw new Error("Webview shortcut storage unavailable");
      }
      return read.call(this, key);
    });
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    expect(element(".window-keybind-button").textContent).toBe("F10");
    expect(backend.initializeShortcuts).toHaveBeenCalledWith("", "");
  });

  it("blocks edits until native preferences finish loading", async () => {
    const loading = deferred<backend.ShortcutSettingsSnapshot>();
    vi.mocked(backend.initializeShortcuts).mockReturnValue(loading.promise);
    await import("./main");
    expect(element<HTMLButtonElement>(".window-keybind-button").disabled).toBe(true);
    element<HTMLButtonElement>(".window-keybind-button").click();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F12", code: "F12" }));
    expect(backend.setWindowShortcut).not.toHaveBeenCalled();
    loading.resolve(saved);
    await loading.promise;
    expect(element<HTMLButtonElement>(".window-keybind-button").disabled).toBe(false);
    expect(element(".window-keybind-button").textContent).toBe("F10");
  });

  it("shows a load error and prevents overwriting unreadable native preferences", async () => {
    vi.mocked(backend.initializeShortcuts).mockRejectedValue(new Error("Could not read saved shortcuts."));
    await import("./main");
    expect(element(".window-keybind-status").textContent).toContain("Could not read saved shortcuts");
    expect(element<HTMLButtonElement>(".window-keybind-button").disabled).toBe(true);
  });

  it.each(["recording", "window"] as const)("keeps %s startup registration errors beside the saved assignment", async (target) => {
    vi.mocked(backend.initializeShortcuts).mockResolvedValue({ ...saved,
      [target === "window" ? "windowError" : "recordingError"]: "Key unavailable",
    });
    await import("./main");
    const { key, status } = controls(target);
    expect(element(key).textContent).toBe(target === "window" ? "F10" : "F9");
    expect(element(status).textContent).toBe("Key unavailable");
    vi.mocked(backend.onAppStateChanged).mock.calls[0][0]({ ...idle, status: "recording", sessionId: 1 });
    expect(element(status).textContent).toBe("Key unavailable");
  });

  it.each(["recording", "window"] as const)("saves a %s assignment only after native persistence succeeds", async (target) => {
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    const { register, key, status } = controls(target);
    const saving = deferred<void>();
    vi.mocked(register).mockReturnValueOnce(saving.promise);
    element<HTMLButtonElement>(key).click();
    for (const value of ["F12", "F11", "Escape"]) {
      window.dispatchEvent(new KeyboardEvent("keydown", { key: value, code: value }));
    }
    expect(register).toHaveBeenCalledExactlyOnceWith("F12");
    expect(element<HTMLButtonElement>(key).disabled).toBe(true);
    expect(element(status).textContent).toBe("Saving shortcut...");
    saving.resolve();
    await saving.promise;
    expect(element(key).textContent).toBe("F12");
    expect(localStorage.getItem("stt.windowShortcut")).toBeNull();
    expect(localStorage.getItem("stt.globalShortcut")).toBeNull();
    expect(backend.toggleBackendRecording).not.toHaveBeenCalled();
  });

  it.each(["recording", "window"] as const)("keeps a %s assignment after failed native saving and allows retry", async (target) => {
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    const { register, key, status } = controls(target);
    vi.mocked(register).mockRejectedValueOnce(new Error("Could not save settings"));
    element<HTMLButtonElement>(key).click();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F12", code: "F12" }));
    await Promise.resolve();
    expect(element(status).textContent).toBe("Could not save settings");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape" }));
    expect(element(key).textContent).toBe(target === "window" ? "F10" : "F9");
    element<HTMLButtonElement>(key).click();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F12", code: "F12" }));
    await Promise.resolve();
    expect(element(key).textContent).toBe("F12");
  });

  it.each(["recording", "window"] as const)("clears only the %s shortcut and keeps it unassigned after reopening", async (target) => {
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    const { register, key, clear } = controls(target);
    const clearing = deferred<void>();
    vi.mocked(register).mockReturnValueOnce(clearing.promise);
    element<HTMLButtonElement>(clear).click();
    expect(register).toHaveBeenCalledExactlyOnceWith("");
    for (const selector of [".keybind-button", ".window-keybind-button", ".clear-keybind-button", ".clear-window-keybind-button"]) {
      expect(element<HTMLButtonElement>(selector).disabled).toBe(true);
    }
    clearing.resolve();
    await clearing.promise;
    expect(element(key).textContent).toBe(target === "window" ? "Set window shortcut" : "Set shortcut");
    vi.mocked(backend.initializeShortcuts).mockResolvedValue({ ...saved, [target]: "" });
    await reopenApp();
    expect(register).not.toHaveBeenCalled();
    expect(element<HTMLButtonElement>(clear).disabled).toBe(true);
    expect(element(controls(target === "window" ? "recording" : "window").key).textContent).toBe(target === "window" ? "F9" : "F10");
  });

  it.each(["recording", "window"] as const)("preserves the %s shortcut after failed clearing and allows retry", async (target) => {
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    const { register, key, clear, status } = controls(target);
    vi.mocked(register).mockRejectedValueOnce(new Error("Could not clear shortcut"));
    element<HTMLButtonElement>(clear).click();
    await Promise.resolve();
    expect(element(key).textContent).toBe(target === "window" ? "F10" : "F9");
    expect(element(status).textContent).toBe("Could not clear shortcut");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F12", code: "F12" }));
    expect(register).toHaveBeenCalledTimes(1);
    element<HTMLButtonElement>(clear).click();
    await Promise.resolve();
    expect(element(status).textContent).toContain("cleared");
  });

  it("moves the capture prompt between controls and Escape preserves the assignment", async () => {
    vi.mocked(backend.initializeShortcuts).mockResolvedValue(saved);
    await import("./main");
    element<HTMLButtonElement>(".window-keybind-button").click();
    element<HTMLButtonElement>(".keybind-button").click();
    expect(element(".window-keybind-status").textContent).not.toContain("Press a shortcut");
    expect(element(".keybind-status").textContent).toContain("Press a shortcut");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", code: "Escape" }));
    expect(element(".keybind-button").textContent).toBe("F9");
    expect(element(".window-keybind-button").textContent).toBe("F10");
    expect(backend.setGlobalShortcut).not.toHaveBeenCalled();
    expect(backend.setWindowShortcut).not.toHaveBeenCalled();
  });
});

describe("organized settings", () => {
  it("keeps a changed provider key editable across renders, closes after saving, and reopens after removal", async () => {
    await startApp("soniox");
    const input = element<HTMLInputElement>(".api-key-input");
    const editor = input.closest<HTMLDetailsElement>("details")!;
    expect(editor.open).toBe(false);
    editor.open = true;
    input.value = "replacement-test-key";
    element<HTMLButtonElement>(".view-toggle-button").click();
    expect(editor.open).toBe(true);
    expect(input.value).toBe("replacement-test-key");
    vi.mocked(backend.saveProviderApiKey).mockResolvedValue(true);
    input.focus();
    element<HTMLButtonElement>(".api-key-save-button").click();
    await vi.advanceTimersByTimeAsync(0);
    expect(document.activeElement).toBe(editor.querySelector("summary"));
    expect(backend.saveProviderApiKey).toHaveBeenCalledWith("soniox", "replacement-test-key");
    expect(editor.open).toBe(false);
    expect(input.value).toBe("");
    element<HTMLButtonElement>(".api-key-delete-button").click();
    await vi.advanceTimersByTimeAsync(0);
    expect(editor.open).toBe(true);
    expect(element<HTMLButtonElement>(".api-key-delete-button").hidden).toBe(true);
  });

  it("keeps live word visibility dependent on live transcript and saves the paste preference", async () => {
    await startApp("soniox");
    const live = element<HTMLInputElement>(".live-transcript-checkbox");
    for (const enabled of [false, true]) {
      live.checked = enabled;
      live.dispatchEvent(new Event("change"));
      expect(element<HTMLElement>(".show-partial-field").hidden).toBe(!enabled);
    }
    const paste = element<HTMLInputElement>(".paste-to-target-checkbox");
    paste.checked = true;
    paste.dispatchEvent(new Event("change"));
    expect(backend.setPasteToTargetBackend).toHaveBeenCalledWith(true);
  });

  it("opens the cleanup editor and focuses its field when setup is requested from Capture", async () => {
    await startApp("soniox");
    const input = element<HTMLInputElement>(".cleanup-api-key-input");
    const editor = input.closest<HTMLDetailsElement>("details")!;
    editor.open = false;
    element<HTMLButtonElement>(".cleanup-setup-button").click();
    expect(element<HTMLElement>(".settings-view").hidden).toBe(false);
    expect(editor.open).toBe(true);
    expect(document.activeElement).toBe(input);
  });
});
