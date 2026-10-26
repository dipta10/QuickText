export const closeCredentialEditor = (input: HTMLInputElement) => {
  const editor = input.closest<HTMLDetailsElement>(".credential-editor");
  if (!editor) return;
  const hadFocus = editor.contains(document.activeElement);
  editor.open = false;
  if (hadFocus) editor.querySelector("summary")?.focus();
};

/** Keep saved secrets out of view while preserving an editor opened by the user. */
export const renderCredentialEditor = (input: HTMLInputElement, configured: boolean) => {
  const editor = input.closest<HTMLDetailsElement>(".credential-editor");
  if (!editor) return;
  const remove = input.closest(".credential-settings")?.querySelector<HTMLButtonElement>(".credential-status-row button");
  if (remove) remove.hidden = !configured;
  const value = String(configured);
  if (editor.dataset.configured !== value) {
    if (configured) closeCredentialEditor(input);
    else editor.open = true;
    editor.dataset.configured = value;
  }
  const summary = editor.querySelector("summary");
  if (summary) summary.textContent = configured ? "Change API key" : "Set up API key";
};
