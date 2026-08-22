export type RecordingState = "idle" | "recording";
export type ShortcutCaptureState = "idle" | "capturing";

export type AppState = {
  recording: RecordingState;
  shortcutCapture: ShortcutCaptureState;
  selectedShortcut: string;
  status: string;
};

export const createAppState = (selectedShortcut: string): AppState => ({
  recording: "idle",
  shortcutCapture: "idle",
  selectedShortcut,
  status: "",
});

export const isRecording = (state: AppState) => state.recording === "recording";

export const isCapturingShortcut = (state: AppState) =>
  state.shortcutCapture === "capturing";

export const startShortcutCapture = (state: AppState): AppState => ({
  ...state,
  shortcutCapture: "capturing",
  status: "Press a shortcut. Esc cancels.",
});

export const cancelShortcutCapture = (state: AppState): AppState => ({
  ...state,
  shortcutCapture: "idle",
  status: "Canceled.",
});

export const toggleRecording = (state: AppState): AppState => ({
  ...state,
  recording: isRecording(state) ? "idle" : "recording",
});

export const saveShortcut = (
  state: AppState,
  selectedShortcut: string,
): AppState => ({
  ...state,
  shortcutCapture: "idle",
  selectedShortcut,
  status: "Saved.",
});

export const setStatus = (state: AppState, status: string): AppState => ({
  ...state,
  status,
});
