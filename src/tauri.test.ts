import { afterEach, describe, expect, it, vi } from "vitest";
import { listen } from "@tauri-apps/api/event";
import { onAppStateChanged, onDeviceFallback, onPartialTranscript } from "./tauri";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetAllMocks();
});

describe("backend event subscriptions", () => {
  it.each([
    ["app state", () => onAppStateChanged(() => {})],
    ["partial transcript", () => onPartialTranscript(() => {})],
    ["device fallback", () => onDeviceFallback(() => {})],
  ] as const)("rejects %s subscriptions asynchronously outside Tauri", async (_name, subscribe) => {
    vi.stubGlobal("window", {});
    // The caller attaches .catch() and continues initializing the app.
    // A synchronous throw would leave the provider picker stuck loading.
    await expect(subscribe()).rejects.toThrow("Run the desktop app");
    expect(listen).not.toHaveBeenCalled();
  });

  it("lets callers handle event registration failures without aborting startup", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    vi.mocked(listen).mockRejectedValue(new Error("Event registration failed"));
    const handled = vi.fn();
    await onAppStateChanged(() => {}).catch(handled);
    expect(handled).toHaveBeenCalledWith(expect.objectContaining({ message: "Event registration failed" }));
  });

  it("forwards backend state events and returns the unsubscribe callback", async () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const unsubscribe = vi.fn();
    vi.mocked(listen).mockResolvedValue(unsubscribe);
    const handler = vi.fn();
    expect(await onAppStateChanged(handler)).toBe(unsubscribe);
    const callback = vi.mocked(listen).mock.calls[0][1];
    const payload = { status: "recording", sessionId: 1, transcript: null, error: null };
    callback({ event: "app-state-changed", id: 1, payload });
    expect(handler).toHaveBeenCalledWith(payload);
  });
});
