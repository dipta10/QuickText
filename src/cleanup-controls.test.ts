// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createAppState, applyBackendSnapshot, type AppState } from "./app-state";
import { createAppView } from "./app-view";
import { connectCleanupControls, renderCleanupControls } from "./cleanup-controls";
import * as backend from "./tauri";

vi.mock("./tauri");
const deferred = <T>() => {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const settle = async () => { await Promise.resolve(); await Promise.resolve(); };

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '<div id="app"></div>';
  vi.mocked(backend.hasCleanupApiKey).mockResolvedValue(false);
  vi.mocked(backend.saveCleanupApiKey).mockResolvedValue(undefined);
  vi.mocked(backend.deleteCleanupApiKey).mockResolvedValue(undefined);
  vi.mocked(backend.cleanAndCopy).mockResolvedValue(undefined);
  vi.mocked(backend.openCleanupKeySetup).mockResolvedValue(undefined);
});

const start = () => {
  const view = createAppView(document.querySelector<HTMLElement>("#app")!);
  let state = applyBackendSnapshot(createAppState("", 300, false, false, false, false, false, "", false, false), {
    status: "transcribed", sessionId: 1, transcript: { text: "Um, do this.", provider: "soniox" }, error: null,
  });
  const update = (next: AppState) => { state = next; renderCleanupControls(view, state); };
  connectCleanupControls(view, () => state, update);
  update(state);
  return { view, update, state: () => state };
};

describe("cleanup settings and actions", () => {
  it("opens the fixed key setup page through the backend rather than navigating QuickText", async () => {
    const { view } = start();
    view.cleanupKeyLink.click();
    await settle();
    expect(backend.openCleanupKeySetup).toHaveBeenCalledTimes(1);
  });
  it("keeps cleanup disabled for an unsaved key, then enables on save and disables on delete", async () => {
    const { view, state } = start();
    await settle();
    expect(view.cleanButton.disabled).toBe(true);
    view.cleanupApiKeyInput.value = "fake-test-key";
    expect(view.cleanButton.disabled).toBe(true);
    view.cleanupApiKeySaveButton.click();
    await settle();
    expect(backend.saveCleanupApiKey).toHaveBeenCalledWith("fake-test-key");
    expect(view.cleanupApiKeyInput.value).toBe("");
    expect(view.cleanButton.disabled).toBe(false);
    expect(view.cleanupSetupButton.hidden).toBe(true);
    view.cleanupApiKeyDeleteButton.click();
    await settle();
    expect(state().cleanupApiKeyConfigured).toBe(false);
    expect(view.cleanButton.disabled).toBe(true);
  });

  it("a delayed startup key check cannot undo a successful save", async () => {
    const pending = deferred<boolean>();
    vi.mocked(backend.hasCleanupApiKey).mockReturnValue(pending.promise);
    const { view, state } = start();
    view.cleanupApiKeyInput.value = "fake-key";
    view.cleanupApiKeySaveButton.click();
    await settle();
    pending.resolve(false);
    await settle();
    expect(state().cleanupApiKeyConfigured).toBe(true);
  });

  it("preserves entered text on save failure and offers setup guidance", async () => {
    vi.mocked(backend.saveCleanupApiKey).mockRejectedValue(new Error("Secure storage unavailable."));
    const { view, state } = start();
    await settle();
    view.cleanupSetupButton.click();
    expect(state().activeView).toBe("settings");
    expect(document.activeElement).toBe(view.cleanupApiKeyInput);
    view.cleanupApiKeyInput.value = "fake-key";
    view.cleanupApiKeySaveButton.click();
    await settle();
    expect(view.cleanupApiKeyInput.value).toBe("fake-key");
    expect(view.cleanupApiKeyStatus.textContent).toContain("Secure storage unavailable.");
    expect(view.cleanButton.disabled).toBe(true);
  });

  it("retains a saved key discovered after a failed save during startup", async () => {
    const pending = deferred<boolean>();
    vi.mocked(backend.hasCleanupApiKey).mockReturnValue(pending.promise);
    vi.mocked(backend.saveCleanupApiKey).mockRejectedValue(new Error("Save failed."));
    const { view, state } = start();
    view.cleanupApiKeyInput.value = "replacement-key";
    view.cleanupApiKeySaveButton.click();
    await settle();
    pending.resolve(true);
    await settle();
    expect(state().cleanupApiKeyConfigured).toBe(true);
    expect(view.cleanButton.disabled).toBe(false);
    expect(view.cleanupApiKeyInput.value).toBe("replacement-key");
    expect(view.cleanupApiKeyStatus.textContent).toBe("Save failed.");
  });

  it("rapid clicks make one backend request and never copy again in the frontend", async () => {
    vi.mocked(backend.hasCleanupApiKey).mockResolvedValue(true);
    const pending = deferred<void>();
    vi.mocked(backend.cleanAndCopy).mockReturnValue(pending.promise);
    const { view, update, state } = start();
    await settle();
    view.cleanButton.click();
    renderCleanupControls(view, state());
    view.cleanButton.click();
    expect(backend.cleanAndCopy).toHaveBeenCalledTimes(1);
    expect(view.cleanButton.disabled).toBe(true);
    update(applyBackendSnapshot(state(), {
      status: "transcribed", sessionId: 1, transcript: { text: "Do this.", provider: "soniox" }, error: null,
      cleanup: { revision: 2, running: false, cleaned: true, message: "Cleaned and copied." },
    }));
    pending.resolve();
    await settle();
    expect(view.restoreButton.hidden).toBe(false);
    expect(view.cleanupStatus.textContent).toBe("Cleaned and copied.");
    expect(backend.copyTextToClipboard).not.toHaveBeenCalled();
  });

  it("does not show an old command error after a new recording starts", async () => {
    vi.mocked(backend.hasCleanupApiKey).mockResolvedValue(true);
    const pending = deferred<void>();
    vi.mocked(backend.cleanAndCopy).mockReturnValue(pending.promise);
    const { view, update, state } = start();
    await settle();
    view.cleanButton.click();
    update(applyBackendSnapshot(state(), { status: "starting", sessionId: 2, transcript: null, error: null }));
    pending.reject(new Error("Old error"));
    await settle();
    expect(state().status).toBe("Starting recording...");
  });
});
