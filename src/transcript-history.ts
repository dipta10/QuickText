import { isBusy, isRecording, type AppState } from "./app-state";
import type { AppView } from "./app-view";
import { getTranscriptHistory, type TranscriptHistorySnapshot } from "./tauri";

const captureInProgress = (state: AppState) => isBusy(state) || isRecording(state);
const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium" });
const timeFormat = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

const dateGroup = (date: Date | null, today: Date, yesterday: Date) => {
  if (!date) return "Earlier · date unavailable";
  const day = date.toDateString();
  if (day === today.toDateString()) return "Today";
  if (day === yesterday.toDateString()) return "Yesterday";
  return dateFormat.format(date);
};

export const createTranscriptHistory = (view: AppView, onSelect: () => void) => {
  let history: TranscriptHistorySnapshot = { entries: [], warning: null };
  let selectedIndex: number | null = null;
  const resetSelection = () => { selectedIndex = null; };
  let requestId = 0;
  let loading = false;
  let renderedEntries: TranscriptHistorySnapshot["entries"] | null = null;
  let renderedDay = "";
  let buttons: HTMLButtonElement[] = [];

  const showCurrent = (state: AppState) =>
    captureInProgress(state) || (selectedIndex === null && Boolean(state.transcript.trim()));

  const renderEntries = (today: Date) => {
    const yesterday = new Date(today);
    yesterday.setDate(today.getDate() - 1);
    let previousGroup: string | null = null;
    const content = document.createDocumentFragment();
    buttons = history.entries.map(({ text, recordedAtMs }, index) => {
      const timestamp = recordedAtMs === null ? NaN : new Date(recordedAtMs).getTime();
      const date = Number.isFinite(timestamp) ? new Date(timestamp) : null;
      const group = dateGroup(date, today, yesterday);
      if (group !== previousGroup) {
        const heading = document.createElement("h3");
        heading.className = "history-day";
        heading.textContent = group;
        content.append(heading);
        previousGroup = group;
      }

      const button = document.createElement("button");
      button.type = "button";
      button.className = "history-entry";
      const preview = document.createElement("span");
      preview.className = "history-preview";
      preview.textContent = text.replace(/\s+/gu, " ").trim().slice(0, 160);
      if (date) {
        const time = document.createElement("time");
        time.dateTime = date.toISOString();
        time.textContent = timeFormat.format(date);
        button.append(time);
      }
      button.append(preview);
      button.addEventListener("click", () => {
        selectedIndex = index;
        onSelect();
      });
      content.append(button);
      return button;
    });
    view.historyList.replaceChildren(content);
    renderedEntries = history.entries;
    renderedDay = today.toDateString();
  };

  return {
    async reload() {
      const latestRequest = ++requestId;
      loading = true;
      try {
        const snapshot = await getTranscriptHistory();
        if (latestRequest === requestId) {
          history = snapshot;
          resetSelection();
        }
      } catch {
        if (latestRequest === requestId) {
          history = { entries: [], warning: "Could not load transcript history." };
          resetSelection();
        }
      } finally {
        if (latestRequest === requestId) loading = false;
      }
    },
    reset: resetSelection,
    displayedText(state: AppState) {
      if (showCurrent(state)) return state.transcript;
      return history.entries[selectedIndex ?? 0]?.text ?? "";
    },
    isViewingSavedTranscript(state: AppState) {
      return !showCurrent(state) && history.entries.length > 0;
    },
    render(state: AppState) {
      view.openHistoryButton.disabled =
        captureInProgress(state) || state.cleanup.running || state.cleanupActionPending;
      if (state.activeView !== "history") return;
      const today = new Date();
      if (renderedEntries !== history.entries || renderedDay !== today.toDateString()) {
        renderEntries(today);
      }
      view.historyList.hidden = loading;
      view.historyEmpty.textContent = loading
        ? "Loading history…"
        : history.entries.length || history.warning ? "" : "No saved transcripts yet.";
      view.historyWarning.textContent = history.warning ?? "";
      for (const [index, button] of buttons.entries()) {
        button.setAttribute("aria-current", String(index === (selectedIndex ?? 0)));
      }
    },
  };
};
