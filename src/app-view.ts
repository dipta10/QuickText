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
  transcriptPlaceholder: HTMLParagraphElement;
  transcriptFinal: HTMLSpanElement;
  transcriptPartial: HTMLSpanElement;
  copyButton: HTMLButtonElement;
  keybindButton: HTMLButtonElement;
  keybindStatus: HTMLParagraphElement;
  shortcutFocusOnStartCheckbox: HTMLInputElement;
  shortcutHideOnStopCheckbox: HTMLInputElement;
  apiKeyInput: HTMLInputElement;
  apiKeySaveButton: HTMLButtonElement;
  apiKeyDeleteButton: HTMLButtonElement;
  apiKeyStatus: HTMLParagraphElement;
  inputDeviceSelect: HTMLSelectElement;
  inputDeviceStatus: HTMLParagraphElement;
  maxRecordingSecondsInput: HTMLInputElement;
  autoCopyCheckbox: HTMLInputElement;
  liveTranscriptCheckbox: HTMLInputElement;
  liveTranscriptField: HTMLLabelElement;
  showPartialCheckbox: HTMLInputElement;
  showPartialField: HTMLLabelElement;
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
          <div class="transcript-text" tabindex="0"
            ><p class="transcript-placeholder">Transcript will appear here.</p
            ><span class="transcript-final"></span
            ><span class="transcript-partial" aria-label="Unconfirmed words"></span
          ></div>
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
          <p class="settings-note">
            The shortcut works from any app: press once to start recording,
            press again to stop. The transcript appears in Capture, ready to
            copy.
          </p>
          <button class="keybind-button" type="button">
            Set shortcut
          </button>
          <p class="keybind-status" role="status"></p>
          <label class="checkbox-field" for="shortcut-focus-on-start">
            <input
              class="shortcut-focus-on-start-checkbox"
              id="shortcut-focus-on-start"
              type="checkbox"
            />
            <span>Focus window when recording starts</span>
          </label>
          <label class="checkbox-field" for="shortcut-hide-on-stop">
            <input
              class="shortcut-hide-on-stop-checkbox"
              id="shortcut-hide-on-stop"
              type="checkbox"
            />
            <span>Hide window when recording stops</span>
          </label>
        </section>

        <section class="settings-section">
          <h2 class="section-title">Microphone</h2>
          <label class="settings-field" for="input-device">
            <span class="settings-field-label">Input device</span>
            <select class="input-device-select" id="input-device"></select>
          </label>
          <p class="input-device-status" role="status"></p>
          <p class="settings-note">
            If the selected microphone is missing when recording starts, the
            system default is used instead.
          </p>
        </section>

        <section class="settings-section">
          <h2 class="section-title">Behavior</h2>
          <label class="settings-field" for="max-recording-seconds">
            <span class="settings-field-label">Maximum recording seconds</span>
            <input
              class="max-recording-seconds-input"
              id="max-recording-seconds"
              type="number"
              min="1"
              step="1"
              inputmode="numeric"
            />
          </label>
          <label class="checkbox-field" for="auto-copy-transcript">
            <input
              class="auto-copy-checkbox"
              id="auto-copy-transcript"
              type="checkbox"
            />
            <span>Automatically copy transcript when done</span>
          </label>
          <label class="checkbox-field live-transcript-field" for="live-transcript">
            <input
              class="live-transcript-checkbox"
              id="live-transcript"
              type="checkbox"
            />
            <span>Show transcript in real time while recording</span>
          </label>
          <label
            class="checkbox-field show-partial-field"
            for="show-partial-transcript"
          >
            <input
              class="show-partial-transcript-checkbox"
              id="show-partial-transcript"
              type="checkbox"
            />
            <span>Show unconfirmed words as they form</span>
          </label>
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
  const transcriptPlaceholder = root.querySelector<HTMLParagraphElement>(
    ".transcript-placeholder",
  );
  const transcriptFinal =
    root.querySelector<HTMLSpanElement>(".transcript-final");
  const transcriptPartial =
    root.querySelector<HTMLSpanElement>(".transcript-partial");
  const copyButton = root.querySelector<HTMLButtonElement>(".copy-button");
  const keybindButton =
    root.querySelector<HTMLButtonElement>(".keybind-button");
  const keybindStatus =
    root.querySelector<HTMLParagraphElement>(".keybind-status");
  const shortcutFocusOnStartCheckbox = root.querySelector<HTMLInputElement>(
    ".shortcut-focus-on-start-checkbox",
  );
  const shortcutHideOnStopCheckbox = root.querySelector<HTMLInputElement>(
    ".shortcut-hide-on-stop-checkbox",
  );
  const apiKeyInput = root.querySelector<HTMLInputElement>(".api-key-input");
  const apiKeySaveButton = root.querySelector<HTMLButtonElement>(
    ".api-key-save-button",
  );
  const apiKeyDeleteButton = root.querySelector<HTMLButtonElement>(
    ".api-key-delete-button",
  );
  const apiKeyStatus =
    root.querySelector<HTMLParagraphElement>(".api-key-status");
  const inputDeviceSelect = root.querySelector<HTMLSelectElement>(
    ".input-device-select",
  );
  const inputDeviceStatus =
    root.querySelector<HTMLParagraphElement>(".input-device-status");
  const maxRecordingSecondsInput = root.querySelector<HTMLInputElement>(
    ".max-recording-seconds-input",
  );
  const autoCopyCheckbox = root.querySelector<HTMLInputElement>(
    ".auto-copy-checkbox",
  );
  const liveTranscriptField =
    root.querySelector<HTMLLabelElement>(".live-transcript-field");
  const liveTranscriptCheckbox = root.querySelector<HTMLInputElement>(
    ".live-transcript-checkbox",
  );
  const showPartialField =
    root.querySelector<HTMLLabelElement>(".show-partial-field");
  const showPartialCheckbox = root.querySelector<HTMLInputElement>(
    ".show-partial-transcript-checkbox",
  );
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
    !transcriptPlaceholder ||
    !transcriptFinal ||
    !transcriptPartial ||
    !copyButton ||
    !keybindButton ||
    !keybindStatus ||
    !shortcutFocusOnStartCheckbox ||
    !shortcutHideOnStopCheckbox ||
    !apiKeyInput ||
    !apiKeySaveButton ||
    !apiKeyDeleteButton ||
    !apiKeyStatus ||
    !inputDeviceSelect ||
    !inputDeviceStatus ||
    !maxRecordingSecondsInput ||
    !autoCopyCheckbox ||
    !liveTranscriptField ||
    !liveTranscriptCheckbox ||
    !showPartialField ||
    !showPartialCheckbox ||
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
    transcriptPlaceholder,
    transcriptFinal,
    transcriptPartial,
    copyButton,
    keybindButton,
    keybindStatus,
    shortcutFocusOnStartCheckbox,
    shortcutHideOnStopCheckbox,
    apiKeyInput,
    apiKeySaveButton,
    apiKeyDeleteButton,
    apiKeyStatus,
    inputDeviceSelect,
    inputDeviceStatus,
    maxRecordingSecondsInput,
    autoCopyCheckbox,
    liveTranscriptField,
    liveTranscriptCheckbox,
    showPartialField,
    showPartialCheckbox,
    bottomStatus,
  };
};
