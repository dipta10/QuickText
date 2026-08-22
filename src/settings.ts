const shortcutStorageKey = "stt.globalShortcut";
const maxRecordingSecondsStorageKey = "stt.maxRecordingSeconds";
const autoCopyTranscriptStorageKey = "stt.autoCopyTranscript";
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
