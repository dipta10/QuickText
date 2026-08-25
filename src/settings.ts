const shortcutStorageKey = "stt.globalShortcut";
const maxRecordingSecondsStorageKey = "stt.maxRecordingSeconds";
const autoCopyTranscriptStorageKey = "stt.autoCopyTranscript";
const shortcutFocusOnStartStorageKey = "stt.shortcutFocusOnStart";
const shortcutHideOnStopStorageKey = "stt.shortcutHideOnStop";
const liveTranscriptStorageKey = "stt.liveTranscript";
const showPartialTranscriptStorageKey = "stt.showPartialTranscript";
const launchOnStartupStorageKey = "stt.launchOnStartup";
export const defaultMaxRecordingSeconds = 300;

export const getGlobalShortcut = (): string => {
  return localStorage.getItem(shortcutStorageKey) ?? "";
};

export const saveGlobalShortcut = (shortcut: string) => {
  localStorage.setItem(shortcutStorageKey, shortcut);
};

export const getMaxRecordingSeconds = (): number => {
  const storedValue = Number(localStorage.getItem(maxRecordingSecondsStorageKey));

  if (!Number.isInteger(storedValue) || storedValue < 1) {
    return defaultMaxRecordingSeconds;
  }

  return storedValue;
};

export const saveMaxRecordingSeconds = (seconds: number) => {
  localStorage.setItem(maxRecordingSecondsStorageKey, seconds.toString());
};

export const getAutoCopyTranscript = (): boolean => {
  return localStorage.getItem(autoCopyTranscriptStorageKey) === "true";
};

export const saveAutoCopyTranscript = (autoCopyTranscript: boolean) => {
  localStorage.setItem(
    autoCopyTranscriptStorageKey,
    autoCopyTranscript.toString(),
  );
};

export const getShortcutFocusOnStart = (): boolean => {
  return localStorage.getItem(shortcutFocusOnStartStorageKey) !== "false";
};

export const saveShortcutFocusOnStart = (focusOnStart: boolean) => {
  localStorage.setItem(shortcutFocusOnStartStorageKey, focusOnStart.toString());
};

export const getShortcutHideOnStop = (): boolean => {
  return localStorage.getItem(shortcutHideOnStopStorageKey) === "true";
};

export const saveShortcutHideOnStop = (hideOnStop: boolean) => {
  localStorage.setItem(shortcutHideOnStopStorageKey, hideOnStop.toString());
};

export const getLiveTranscript = (): boolean => {
  return localStorage.getItem(liveTranscriptStorageKey) !== "false";
};

export const saveLiveTranscript = (liveTranscript: boolean) => {
  localStorage.setItem(liveTranscriptStorageKey, liveTranscript.toString());
};

export const getShowPartialTranscript = (): boolean => {
  return localStorage.getItem(showPartialTranscriptStorageKey) !== "false";
};

export const saveShowPartialTranscript = (showPartial: boolean) => {
  localStorage.setItem(showPartialTranscriptStorageKey, showPartial.toString());
};

export const getLaunchOnStartup = (): boolean => {
  return localStorage.getItem(launchOnStartupStorageKey) === "true";
};

export const saveLaunchOnStartup = (launchOnStartup: boolean) => {
  localStorage.setItem(launchOnStartupStorageKey, launchOnStartup.toString());
};
