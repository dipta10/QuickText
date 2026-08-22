const shortcutStorageKey = "stt.globalShortcut";

export const getGlobalShortcut = (): string => {
  return localStorage.getItem(shortcutStorageKey) ?? "";
};

export const saveGlobalShortcut = (shortcut: string) => {
  localStorage.setItem(shortcutStorageKey, shortcut);
};
