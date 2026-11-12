import type { SpeechSnapshot } from "./tauri";
export const emptySpeechSnapshot = (): SpeechSnapshot => ({ status: "idle", sessionId: 0, revision: 0, error: null });
export const acceptSpeechSnapshot = (current: SpeechSnapshot, next: SpeechSnapshot): SpeechSnapshot =>
  next.revision > current.revision ? next : current;
export const speechIsActive = (snapshot: SpeechSnapshot) => snapshot.status === "preparing" || snapshot.status === "playing";
export const speechLabel = (snapshot: SpeechSnapshot): string => ({
  idle: "Ready to read", preparing: "Preparing audio…", playing: "Reading aloud", completed: "Reading complete", error: "Couldn’t read aloud",
})[snapshot.status];
