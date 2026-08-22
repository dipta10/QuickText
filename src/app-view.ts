export type AppView = {
  recordButton: HTMLButtonElement;
  keybindButton: HTMLButtonElement;
  keybindStatus: HTMLParagraphElement;
};

const appTemplate = `
  <section class="app-shell" aria-label="Speech to Text">
    <button class="record-button" type="button" aria-label="Start recording">
      Record
    </button>

    <div class="keybind-panel">
      <span class="keybind-label">Keybind</span>
      <button class="keybind-button" type="button">
        Set shortcut
      </button>
      <p class="keybind-status" role="status"></p>
    </div>
  </section>
`;

export const createAppView = (root: HTMLElement): AppView => {
  root.innerHTML = appTemplate;

  const recordButton =
    root.querySelector<HTMLButtonElement>(".record-button");
  const keybindButton =
    root.querySelector<HTMLButtonElement>(".keybind-button");
  const keybindStatus =
    root.querySelector<HTMLParagraphElement>(".keybind-status");

  if (!recordButton || !keybindButton || !keybindStatus) {
    throw new Error("App controls were not found");
  }

  return {
    recordButton,
    keybindButton,
    keybindStatus,
  };
};
