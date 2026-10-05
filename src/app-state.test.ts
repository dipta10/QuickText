import { describe, expect, it } from "vitest";

import {
  applyBackendSnapshot,
  applyPartialTranscript,
  canToggleRecording,
  createAppState,
  type BackendAppSnapshot,
  setActiveProvider,
  setProviderApiKeyPresence,
  setProviderSelectionStatus,
} from "./app-state";

const snapshot = (
  status: BackendAppSnapshot["status"],
  error: BackendAppSnapshot["error"] = null,
  sessionId: number | null = null,
): BackendAppSnapshot => ({ status, sessionId, transcript: null, error });

describe("recording-session transcript lifecycle", () => {
  it("clears live text when finalization times out", () => {
    let state = createAppState(
      "",
      300,
      false,
      false,
      false,
      true,
      true,
      "",
      false,
      false,
    );

    state = applyBackendSnapshot(state, snapshot("recording", null, 1));
    state = applyPartialTranscript(state, {
      recordingSessionId: 1,
      providerSessionId: 1,
      finalText: "words from the failed recording",
      partialText: "still forming",
    });
    state = applyBackendSnapshot(state, snapshot("stopping", null, 1));
    state = applyBackendSnapshot(
      state,
      snapshot("error", {
        type: "provider_unavailable",
        message: "Soniox finalization timed out.",
      }, 1),
    );

    expect(state.transcript).toBe("");
    expect(state.partialTranscript).toBe("");
  });

  it("starts a new recording without the previous transcript", () => {
    let state = createAppState(
      "",
      300,
      false,
      false,
      false,
      true,
      true,
      "",
      false,
      false,
    );

    state = applyBackendSnapshot(state, {
      status: "transcribed",
      sessionId: 1,
      transcript: {
        text: "words from the previous recording",
        provider: "soniox",
      },
      error: null,
    });
    state = applyBackendSnapshot(state, snapshot("starting", null, 2));

    expect(state.transcript).toBe("");
    expect(state.partialTranscript).toBe("");
  });

  it.each(["recording", "stopping"] as const)(
    "clears the previous transcript when a new %s reply precedes starting",
    (status) => {
      const completed = applyBackendSnapshot(
        createAppState("", 300, false, false, false, false, false, "", false, false),
        {
          ...snapshot("transcribed", null, 1),
          transcript: { text: "previous dictation", provider: "deepgram" },
        },
      );
      const next = applyBackendSnapshot(completed, snapshot(status, null, 2));
      expect(next.transcript).toBe("");
      expect(next.partialTranscript).toBe("");
      expect(applyBackendSnapshot(next, snapshot("starting", null, 2))).toBe(next);
    },
  );

  it("resets live text and the clock when recording from a newer session arrives first", () => {
    let previous = applyBackendSnapshot(
      createAppState("", 300, false, false, false, true, true, "", false, false),
      snapshot("recording", null, 1),
    );
    previous = applyPartialTranscript(previous, {
      recordingSessionId: 1,
      providerSessionId: 1,
      finalText: "previous live text",
      partialText: "previous draft",
    });
    previous = { ...previous, recordingStartedAt: 1 };
    const next = applyBackendSnapshot(previous, snapshot("recording", null, 2));
    expect(next.transcript).toBe("");
    expect(next.partialTranscript).toBe("");
    expect(next.recordingStartedAt).toBeGreaterThan(1);
  });

  it("preserves current live text and the clock on a repeated recording snapshot", () => {
    let current = applyBackendSnapshot(
      createAppState("", 300, false, false, false, true, true, "", false, false),
      snapshot("recording", null, 1),
    );
    current = applyPartialTranscript(current, {
      recordingSessionId: 1,
      providerSessionId: 1,
      finalText: "current live text",
      partialText: "current draft",
    });
    const next = applyBackendSnapshot(current, snapshot("recording", null, 1));
    expect(next.transcript).toBe("current live text");
    expect(next.partialTranscript).toBe("current draft");
    expect(next.recordingStartedAt).toBe(current.recordingStartedAt);
  });

  it("ignores delayed transcript updates from an older recording", () => {
    let state = createAppState(
      "",
      300,
      false,
      false,
      false,
      true,
      true,
      "",
      false,
      false,
    );

    state = applyBackendSnapshot(state, snapshot("recording", null, 2));
    const unchanged = applyPartialTranscript(state, {
      recordingSessionId: 1,
      providerSessionId: 9,
      finalText: "stale recording",
      partialText: "stale",
    });

    expect(unchanged).toBe(state);
  });

  it("routes missing credentials to the provider named by the backend", () => {
    const state = createAppState(
      "",
      300,
      false,
      false,
      false,
      true,
      true,
      "",
      false,
      false,
    );

    const next = applyBackendSnapshot(
      state,
      snapshot("error", {
        type: "missing_api_key",
        provider: "deepgram",
        message: "Add your Deepgram API key before recording.",
      }),
    );

    expect(next.activeView).toBe("settings");
    expect(next.activeProvider).toBe("deepgram");
    expect(next.providerSelectionStatus).toBe("ready");
    expect(next.providerApiKeyStatus.deepgram).toBe(
      "Add your Deepgram API key before recording.",
    );
    expect(next.providerApiKeyStatus.soniox).toBe("");
  });
  it("keeps the two providers' credential status separate", () => {
    let state = createAppState("", 300, false, false, false, true, true, "", false, false);
    state = setProviderApiKeyPresence(state, "soniox", true, "Saved Soniox");
    state = setProviderApiKeyPresence(state, "deepgram", true, "Saved Deepgram");
    state = setProviderApiKeyPresence(state, "soniox", false, "Deleted Soniox");
    state = setActiveProvider(state, "deepgram");
    expect(state.providerApiKeyConfigured).toEqual({soniox: false, deepgram: true});
    expect(state.providerApiKeyStatus.deepgram).toBe("Saved Deepgram");
    expect(state.activeProvider).toBe("deepgram");
  });

  it.each(["starting", "stopping", "transcribed", "error", "idle"] as const)(
    "ignores a partial update while %s",
    (status) => {
      const state = applyBackendSnapshot(
        createAppState("", 300, false, false, false, true, true, "", false, false),
        snapshot(status, null, 1),
      );
      expect(applyPartialTranscript(state, {
        recordingSessionId: 1, providerSessionId: 1,
        finalText: "late words", partialText: "draft",
      })).toBe(state);
    },
  );

  it.each(["starting", "recording", "stopping", "transcribed", "error"] as const)(
    "ignores a delayed %s snapshot from an older recording",
    (status) => {
      const state = applyBackendSnapshot(
        createAppState("", 300, false, false, false, true, true, "", false, false),
        snapshot("recording", null, 2),
      );
      expect(applyBackendSnapshot(state, snapshot(status, null, 1))).toBe(state);
    },
  );

  it("ignores an initial idle response received after recording starts", () => {
    const state = applyBackendSnapshot(
      createAppState("", 300, false, false, false, true, true, "", false, false),
      snapshot("recording", null, 1),
    );
    expect(applyBackendSnapshot(state, snapshot("idle"))).toBe(state);
  });

  it("does not resume live updates when an older recording reply arrives after stop", () => {
    const state = applyBackendSnapshot(
      createAppState("", 300, false, false, false, true, true, "", false, false),
      snapshot("stopping", null, 1),
    );
    expect(applyBackendSnapshot(state, snapshot("recording", null, 1))).toBe(state);
  });

  it("allows paste failure after completion but rejects an older successful reply", () => {
    let state = applyBackendSnapshot(
      createAppState("", 300, false, false, false, true, true, "", false, false),
      { ...snapshot("transcribed", null, 1), transcript: {text: "keep this transcript", provider: "deepgram"} },
    );
    state = applyBackendSnapshot(state, {
      ...snapshot("error", {type: "paste_failed", message: "Paste failed"}, 1),
      transcript: {text: "keep this transcript", provider: "deepgram"},
    });
    expect(state.recording).toBe("error");
    expect(state.transcript).toBe("keep this transcript");
    expect(applyBackendSnapshot(state, snapshot("transcribed", null, 1))).toBe(state);
    expect(applyBackendSnapshot(state, snapshot("starting", null, 2)).recording).toBe("starting");
  });

});

describe("provider selection recovery", () => {
  it("blocks capture until the saved provider has loaded", () => {
    const state = createAppState("", 300, false, false, false, true, true, "", false, false);
    expect(canToggleRecording(state)).toBe(false);
    expect(canToggleRecording(setActiveProvider(state, "deepgram"))).toBe(true);
  });

  it.each(["soniox", "deepgram"] as const)(
    "recovers an invalid selection by explicitly choosing %s",
    (provider) => {
      const failed = setProviderSelectionStatus(
        createAppState("", 300, false, false, false, true, true, "", false, false),
        "error",
      );
      expect(canToggleRecording(failed)).toBe(false);
      const recovered = setActiveProvider(failed, provider);
      expect(recovered.activeProvider).toBe(provider);
      expect(recovered.providerSelectionStatus).toBe("ready");
      expect(canToggleRecording(recovered)).toBe(true);
    },
  );

  it("keeps Stop available if provider loading fails during an existing recording", () => {
    const state = applyBackendSnapshot(
      createAppState("", 300, false, false, false, true, true, "", false, false),
      snapshot("recording", null, 1),
    );
    expect(canToggleRecording(setProviderSelectionStatus(state, "error"))).toBe(true);
    expect(canToggleRecording(applyBackendSnapshot(state, snapshot("stopping", null, 1)))).toBe(false);
  });
});
