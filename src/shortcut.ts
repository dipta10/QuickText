export type ShortcutResult =
  | { ok: true; shortcut: string }
  | { ok: false; message?: string };

const modifierKeys = new Set(["Control", "Shift", "Alt", "Meta"]);

const namedKeys: Record<string, string> = {
  ArrowDown: "ArrowDown",
  ArrowLeft: "ArrowLeft",
  ArrowRight: "ArrowRight",
  ArrowUp: "ArrowUp",
  Backspace: "Backspace",
  Delete: "Delete",
  Enter: "Enter",
  Escape: "Escape",
  Space: "Space",
  Tab: "Tab",
};

const formatKey = (event: KeyboardEvent): string | null => {
  if (/^Key[A-Z]$/.test(event.code)) {
    return event.code.replace("Key", "");
  }

  if (/^Digit[0-9]$/.test(event.code)) {
    return event.code.replace("Digit", "");
  }

  if (/^F\d{1,2}$/.test(event.code)) {
    return event.code;
  }

  return namedKeys[event.code] ?? null;
};

export const formatShortcut = (event: KeyboardEvent): ShortcutResult => {
  const key = formatKey(event);

  if (!key || modifierKeys.has(event.key)) {
    return { ok: false };
  }

  if (key === "Escape") {
    return { ok: true, shortcut: "Escape" };
  }

  const modifiers: string[] = [];

  if (event.ctrlKey || event.metaKey) {
    modifiers.push("CmdOrCtrl");
  }

  if (event.altKey) {
    modifiers.push("Alt");
  }

  if (event.shiftKey) {
    modifiers.push("Shift");
  }

  if (modifiers.length === 0) {
    return { ok: false, message: "Use at least one modifier." };
  }

  return { ok: true, shortcut: [...modifiers, key].join("+") };
};
