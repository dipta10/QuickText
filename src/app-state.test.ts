import { describe, expect, it } from "vitest";

import {
  applyBackendSnapshot,
  applyPartialTranscript,
  createAppState,
  type BackendAppSnapshot,
} from "./app-state";

const snapshot = (
  status: BackendAppSnapshot["status"],
  error: BackendAppSnapshot["error"] = null,
): BackendAppSnapshot => ({ status, transcript: null, error });

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

    state = applyBackendSnapshot(state, snapshot("recording"));
    state = applyPartialTranscript(state, {
      final_text: "words from the failed recording",
      partial_text: "still forming",
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
});
