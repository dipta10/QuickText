import { isBusy, isRecording, type AppState } from "./app-state";
import type { AppView } from "./app-view";
import { getTranscriptHistory, type TranscriptHistorySnapshot } from "./tauri";

const captureInProgress = (state: AppState) => isBusy(state) || isRecording(state);

export const createTranscriptHistory = () => {
  let history: TranscriptHistorySnapshot = { entries: [], warning: null };
  let position = 0;
  let requestId = 0;
  let loading = false;

  const showCurrent = (state: AppState) =>
    captureInProgress(state) || (position === 0 && Boolean(state.transcript.trim()));

  const navigationDisabled = (state: AppState) =>
    loading || captureInProgress(state) || state.cleanup.running || state.cleanupActionPending;

  return {
    async reload() {
      const latestRequest = ++requestId;
      loading = true;
      try {
        const snapshot = await getTranscriptHistory();
        if (latestRequest === requestId) {
          history = snapshot;
          position = Math.max(0, Math.min(position, history.entries.length - 1));
        }
      } catch {
        if (latestRequest === requestId) {
          history = { entries: [], warning: "Could not load transcript history." };
          position = 0;
        }
      } finally {
        if (latestRequest === requestId) loading = false;
      }
    },
    reset() {
      position = 0;
    },
    move(direction: -1 | 1, state: AppState) {
      const next = position + direction;
      if (navigationDisabled(state) || next < 0 || next >= history.entries.length) return;
      position = next;
    },
    displayedText(state: AppState) {
      if (showCurrent(state)) return state.transcript;
      return history.entries[position] ?? "";
    },
    isViewingSavedTranscript(state: AppState) {
      return !showCurrent(state) && position < history.entries.length;
    },
    render(view: AppView, state: AppState) {
      const count = history.entries.length;
      const disabled = navigationDisabled(state);
      view.historyNav.hidden = count === 0 || loading || captureInProgress(state);
      const label = view.historyNav.hidden ? "" : `${position + 1} of ${count}`;
      if (view.historyPosition.textContent !== label) view.historyPosition.textContent = label;
      view.olderTranscriptButton.disabled = disabled || position >= count - 1;
      view.newerTranscriptButton.disabled = disabled || position === 0;
      view.historyWarning.textContent = history.warning ?? "";
    },
  };
};
