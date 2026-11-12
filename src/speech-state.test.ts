import { describe, expect, it } from "vitest";
import { acceptSpeechSnapshot, speechIsActive } from "./speech-state";
import type { SpeechSnapshot } from "./tauri";
describe("backend reading state", () => {
  it("rejects late query results, stopped-stream updates, and replies from older readings", () => {
    const current: SpeechSnapshot = {status:"preparing",sessionId:2,revision:5,error:null};
    expect(acceptSpeechSnapshot(current,{status:"playing",sessionId:1,revision:3,error:null})).toBe(current);
    expect(acceptSpeechSnapshot(current,{status:"idle",sessionId:0,revision:0,error:null})).toBe(current);
    const stopped: SpeechSnapshot = {status:"idle",sessionId:2,revision:6,error:null};
    expect(acceptSpeechSnapshot(stopped,current)).toBe(stopped);
    expect(speechIsActive(current)).toBe(true);
    expect(speechIsActive(stopped)).toBe(false);
  });
});
