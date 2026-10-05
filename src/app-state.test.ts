import { describe, expect, it } from "vitest";

import {
  applyBackendSnapshot,
  applyPartialTranscript,
  createAppState,
  type BackendAppSnapshot,
  setActiveProvider,
  setProviderApiKeyPresence,
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
    state = applyBackendSnapshot(state, snapshot("stopping"));
    state = applyBackendSnapshot(
      state,
      snapshot("error", {
        type: "provider_unavailable",
        message: "Soniox finalization timed out.",
      }),
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
    state = applyBackendSnapshot(state, snapshot("starting"));

    expect(state.transcript).toBe("");
    expect(state.partialTranscript).toBe("");
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

});
