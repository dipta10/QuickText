import type { AppState, AudioLevelUpdate } from "./app-state";

type RecordingIdentity = Pick<AppState, "recording" | "activeSessionId">;

// Map amplitude logarithmically so ordinary speech remains visible. This is
// relative digital input level (-60 to 0 dBFS), not calibrated sound pressure.
export const displayAudioLevel = (amplitude: number): number => {
  if (!Number.isFinite(amplitude) || amplitude <= 0) return 0;
  return Math.max(0, Math.min(1, (20 * Math.log10(amplitude) + 60) / 60));
};

export const createAudioLevelMeter = (meter: HTMLElement, bars: HTMLElement[]) => {
  let sessionId: number | null = null;
  let history = bars.map(() => 0);

  const draw = (level: number) => {
    bars.forEach((bar, index) => {
      bar.style.height = `${3 + 25 * history[index]}px`;
    });
    meter.setAttribute("aria-valuenow", String(Math.round(level * 100)));
  };

  const sync = (state: RecordingIdentity) => {
    const currentSession = state.recording === "recording" ? state.activeSessionId : null;
    if (currentSession !== sessionId || currentSession === null) {
      sessionId = currentSession;
      history = bars.map(() => 0);
      draw(0);
    }
  };

  const accept = (state: RecordingIdentity, update: AudioLevelUpdate) => {
    sync(state);
    if (sessionId === null || sessionId !== update.recordingSessionId) return;
    const level = displayAudioLevel(update.level);
    history = [...history.slice(1), level];
    draw(level);
  };

  draw(0);
  return { sync, accept };
};
