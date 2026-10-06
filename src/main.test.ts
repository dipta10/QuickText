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

beforeEach(() => {
  vi.resetModules();
  vi.resetAllMocks();
  vi.useFakeTimers();
  vi.spyOn(window, "addEventListener");
  localStorage.clear();
  document.body.innerHTML = '<div id="app"></div>';
  vi.mocked(backend.getAppState).mockResolvedValue(idle);
  vi.mocked(backend.getTranscriptionSettings).mockResolvedValue({ activeProvider: "deepgram" });
  vi.mocked(backend.hasProviderApiKey).mockResolvedValue(true);
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
    expect(element<HTMLButtonElement>(".record-button").textContent).toBe("Stop");
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
