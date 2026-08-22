import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const setGlobalShortcutCommand = "set_global_shortcut";
const globalShortcutPressedEvent = "global-shortcut-pressed";

export const setGlobalShortcut = async (shortcut: string) => {
  await invoke(setGlobalShortcutCommand, { shortcut });
};

export const onGlobalShortcutPressed = (handler: () => void) => {
  return listen<string>(globalShortcutPressedEvent, () => {
    handler();
  });
};
