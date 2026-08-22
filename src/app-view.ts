export type AppView = {
  statusChip: HTMLSpanElement;
  viewToggleButton: HTMLButtonElement;
  captureView: HTMLElement;
  settingsView: HTMLElement;
  recordButton: HTMLButtonElement;
  recordStatus: HTMLParagraphElement;
  recordingTimer: HTMLSpanElement;
  activityIndicator: HTMLSpanElement;
  transcriptText: HTMLDivElement;
  copyButton: HTMLButtonElement;
  keybindButton: HTMLButtonElement;
  keybindStatus: HTMLParagraphElement;
  apiKeyInput: HTMLInputElement;
  apiKeySaveButton: HTMLButtonElement;
  apiKeyDeleteButton: HTMLButtonElement;
  apiKeyStatus: HTMLParagraphElement;
  bottomStatus: HTMLParagraphElement;
};

const appTemplate = `
  <section class="app-shell" aria-label="Speech to Text">
    <header class="top-bar">
      <div class="brand-block">
        <h1 class="app-title">QuickText</h1>
        <span class="status-chip">Ready</span>
      </div>
      <button class="view-toggle-button" type="button" aria-label="Open settings">
        Settings
      </button>
    </header>

    <main class="main-content">
      <section class="capture-view" aria-label="Capture">
        <div class="recording-controls">
          <button class="record-button" type="button" aria-label="Start recording">
            Record
          </button>
          <div class="recording-meta">
            <p class="record-status">Ready</p>
            <span class="recording-timer">00:00</span>
            <span class="activity-indicator" aria-hidden="true"></span>
          </div>
        </div>

        <section class="transcript-panel" aria-label="Transcript">
          <div class="transcript-toolbar">
            <h2 class="section-title">Transcript</h2>
            <button class="copy-button" type="button">Copy</button>
          </div>
          <div class="transcript-text" tabindex="0">
            Transcript will appear here.
          </div>
        </section>
      </section>

      <section class="settings-view" aria-label="Settings" hidden>
        <section class="settings-section">
          <h2 class="section-title">Soniox</h2>
          <label class="api-key-label" for="soniox-api-key">API key</label>
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
        </section>

        <section class="settings-section">
          <h2 class="section-title">Shortcut</h2>
          <button class="keybind-button" type="button">
            Set shortcut
          </button>
          <p class="keybind-status" role="status"></p>
        </section>

        <section class="settings-section">
          <h2 class="section-title">Behavior</h2>
          <p class="settings-note">Manual copy after transcription.</p>
          <p class="settings-note">Maximum recording: 5 minutes.</p>
        </section>

        <section class="settings-section">
          <h2 class="section-title">App</h2>
          <p class="settings-note">
            Closing the window will hide QuickText when tray mode is connected.
          </p>
        </section>
      </section>
    </main>

    <p class="bottom-status" role="status"></p>
  </section>
`;

export const createAppView = (root: HTMLElement): AppView => {
  root.innerHTML = appTemplate;

  const statusChip = root.querySelector<HTMLSpanElement>(".status-chip");
  const viewToggleButton = root.querySelector<HTMLButtonElement>(
    ".view-toggle-button",
  );
  const captureView = root.querySelector<HTMLElement>(".capture-view");
  const settingsView = root.querySelector<HTMLElement>(".settings-view");
  const recordButton =
    root.querySelector<HTMLButtonElement>(".record-button");
  const recordStatus =
    root.querySelector<HTMLParagraphElement>(".record-status");
  const recordingTimer =
    root.querySelector<HTMLSpanElement>(".recording-timer");
  const activityIndicator =
    root.querySelector<HTMLSpanElement>(".activity-indicator");
  const transcriptText =
    root.querySelector<HTMLDivElement>(".transcript-text");
  const copyButton = root.querySelector<HTMLButtonElement>(".copy-button");
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
  const bottomStatus =
    root.querySelector<HTMLParagraphElement>(".bottom-status");

  if (
    !statusChip ||
    !viewToggleButton ||
    !captureView ||
    !settingsView ||
    !recordButton ||
    !recordStatus ||
    !recordingTimer ||
    !activityIndicator ||
    !transcriptText ||
    !copyButton ||
    !keybindButton ||
    !keybindStatus ||
    !apiKeyInput ||
    !apiKeySaveButton ||
    !apiKeyDeleteButton ||
    !apiKeyStatus ||
    !bottomStatus
  ) {
    throw new Error("App controls were not found");
  }

  return {
    statusChip,
    viewToggleButton,
    captureView,
    settingsView,
    recordButton,
    recordStatus,
    recordingTimer,
    activityIndicator,
    transcriptText,
    copyButton,
    keybindButton,
    keybindStatus,
    apiKeyInput,
    apiKeySaveButton,
    apiKeyDeleteButton,
    apiKeyStatus,
    bottomStatus,
  };
};
