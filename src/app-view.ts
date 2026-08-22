export type AppView = {
  recordButton: HTMLButtonElement;
  transcriptOutput: HTMLTextAreaElement;
  keybindButton: HTMLButtonElement;
  keybindStatus: HTMLParagraphElement;
  apiKeyInput: HTMLInputElement;
  apiKeySaveButton: HTMLButtonElement;
  apiKeyDeleteButton: HTMLButtonElement;
  apiKeyStatus: HTMLParagraphElement;
};

const appTemplate = `
  <section class="app-shell" aria-label="Speech to Text">
    <button class="record-button" type="button" aria-label="Start recording">
      Record
    </button>

    <textarea
      class="transcript-output"
      aria-label="Transcript"
      readonly
      placeholder="Transcript will appear here"
    ></textarea>

    <div class="keybind-panel">
      <span class="keybind-label">Keybind</span>
      <button class="keybind-button" type="button">
        Set shortcut
      </button>
      <p class="keybind-status" role="status"></p>
    </div>

    <div class="api-key-panel">
      <label class="api-key-label" for="soniox-api-key">Soniox API key</label>
      <input
        class="api-key-input"
        id="soniox-api-key"
        type="password"
        autocomplete="off"
        placeholder="Paste key"
      />
      <div class="api-key-actions">
        <button class="api-key-save-button" type="button">Save</button>
        <button class="api-key-delete-button" type="button">Delete</button>
      </div>
      <p class="api-key-status" role="status"></p>
    </div>
  </section>
`;

export const createAppView = (root: HTMLElement): AppView => {
  root.innerHTML = appTemplate;

  const recordButton =
    root.querySelector<HTMLButtonElement>(".record-button");
  const transcriptOutput =
    root.querySelector<HTMLTextAreaElement>(".transcript-output");
  const keybindButton =
    root.querySelector<HTMLButtonElement>(".keybind-button");
  const keybindStatus =
    root.querySelector<HTMLParagraphElement>(".keybind-status");
  const apiKeyInput = root.querySelector<HTMLInputElement>(".api-key-input");
  const apiKeySaveButton = root.querySelector<HTMLButtonElement>(
    ".api-key-save-button",
  );
  const apiKeyDeleteButton = root.querySelector<HTMLButtonElement>(
    ".api-key-delete-button",
  );
  const apiKeyStatus =
    root.querySelector<HTMLParagraphElement>(".api-key-status");

  if (
    !recordButton ||
    !transcriptOutput ||
    !keybindButton ||
    !keybindStatus ||
    !apiKeyInput ||
    !apiKeySaveButton ||
    !apiKeyDeleteButton ||
    !apiKeyStatus
  ) {
    throw new Error("App controls were not found");
  }

  return {
    recordButton,
    transcriptOutput,
    keybindButton,
    keybindStatus,
    apiKeyInput,
    apiKeySaveButton,
    apiKeyDeleteButton,
    apiKeyStatus,
  };
};
