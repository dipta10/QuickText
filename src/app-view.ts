export type AppView = {
  cleanButton: HTMLButtonElement;
  restoreButton: HTMLButtonElement;
  cleanupSetupButton: HTMLButtonElement;
  cleanupFeedback: HTMLDivElement;
  cleanupStatus: HTMLParagraphElement;
  cleanupApiKeyInput: HTMLInputElement;
  cleanupApiKeySaveButton: HTMLButtonElement;
  cleanupApiKeyDeleteButton: HTMLButtonElement;
  cleanupApiKeyStatus: HTMLParagraphElement;
  cleanupKeyLink: HTMLAnchorElement;

  statusChip: HTMLSpanElement;
  viewToggleButton: HTMLButtonElement;
  captureView: HTMLElement;
  settingsView: HTMLElement;
  recordButton: HTMLButtonElement;
  providerLabel: HTMLSpanElement;
  deviceNotice: HTMLParagraphElement;
  recordingTimer: HTMLSpanElement;
  activityIndicator: HTMLSpanElement;
  audioLevelMeter: HTMLElement;
  audioLevelBars: HTMLElement[];
  transcriptText: HTMLDivElement;
  transcriptPlaceholder: HTMLParagraphElement;
  transcriptFinal: HTMLSpanElement;
  transcriptPartial: HTMLSpanElement;
  copyButton: HTMLButtonElement;
  keybindButton: HTMLButtonElement;
  keybindStatus: HTMLParagraphElement;
  windowKeybindButton: HTMLButtonElement;
  windowKeybindStatus: HTMLParagraphElement;
  shortcutFocusOnStartCheckbox: HTMLInputElement;
  shortcutHideOnStopCheckbox: HTMLInputElement;
  providerSelect: HTMLSelectElement;
  providerStatus: HTMLParagraphElement;
  sonioxSection: HTMLElement;
  apiKeyInput: HTMLInputElement;
  apiKeySaveButton: HTMLButtonElement;
  apiKeyDeleteButton: HTMLButtonElement;
  apiKeyStatus: HTMLParagraphElement;
  deepgramSection: HTMLElement;
  deepgramApiKeyInput: HTMLInputElement;
  deepgramApiKeySaveButton: HTMLButtonElement;
  deepgramApiKeyDeleteButton: HTMLButtonElement;
  deepgramApiKeyStatus: HTMLParagraphElement;
  transcriptionDescriptionInput: HTMLTextAreaElement;
  transcriptionDescriptionSaveButton: HTMLButtonElement;
  transcriptionDescriptionStatus: HTMLParagraphElement;
  transcriptionTermsInput: HTMLTextAreaElement;
  transcriptionTermsSaveButton: HTMLButtonElement;
  transcriptionTermsStatus: HTMLParagraphElement;
  languagePicker: HTMLDetailsElement;
  languagePickerSummary: HTMLSpanElement;
  languageSearchInput: HTMLInputElement;
  languageOptions: HTMLDivElement;
  clearLanguagesButton: HTMLButtonElement;
  languageStatus: HTMLParagraphElement;
  inputDeviceSelect: HTMLSelectElement;
  inputDeviceStatus: HTMLParagraphElement;
  maxRecordingSecondsInput: HTMLInputElement;
  autoCopyCheckbox: HTMLInputElement;
  liveTranscriptCheckbox: HTMLInputElement;
  liveTranscriptField: HTMLLabelElement;
  showPartialCheckbox: HTMLInputElement;
  showPartialField: HTMLLabelElement;
  pasteToTargetCheckbox: HTMLInputElement;
  launchOnStartupCheckbox: HTMLInputElement;
  launchOnStartupStatus: HTMLParagraphElement;
  appVersion: HTMLSpanElement;
  buildId: HTMLSpanElement;
  sourceRevision: HTMLSpanElement;
  copyBuildInfoButton: HTMLButtonElement;
  buildInfoStatus: HTMLParagraphElement;
  bottomStatus: HTMLParagraphElement;
  shortcutHint: HTMLSpanElement;
};

const appTemplate = `
  <section class="app-shell" aria-label="Speech to Text">
    <header class="top-bar">
      <div class="brand-block">
        <svg class="brand-mark" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M3 9v6m4-10v14m5-11v8m5-14v20m4-14v8" />
        </svg>
        <h1 class="app-title">QuickText</h1>
      </div>
      <div class="top-bar-actions">
        <span class="status-chip">Ready</span>
        <button class="view-toggle-button" type="button" aria-label="Open settings">
          Settings
        </button>
      </div>
    </header>

    <main class="main-content">
      <section class="capture-view" aria-label="Capture">
        <div class="recording-controls">
          <span class="capture-label">Capture / microphone</span>
          <span class="provider-label">Provider: Soniox</span>
          <button class="record-button" type="button" aria-label="Start recording">
            Start recording
          </button>
          <div class="recording-meta">
            <span class="recording-timer">00:00</span>
            <span class="activity-indicator" aria-hidden="true"></span>
          </div>
        </div>
        <div class="capture-activity">
          <div class="activity-bars" role="meter" aria-label="Microphone input level"
            aria-valuemin="0" aria-valuemax="100" aria-valuenow="0"
          >${"<i aria-hidden=\"true\"></i>".repeat(48)}</div>
          <span class="activity-label" aria-hidden="true">Input level</span>
        </div>
        <p class="device-notice" role="status"></p>

        <section class="transcript-panel" aria-label="Transcript">
          <div class="transcript-toolbar">
            <h2 class="section-title">Transcript</h2>
            <div class="transcript-actions">
              <button class="clean-button" type="button" disabled>Clean &amp; Copy</button>
              <button class="copy-button" type="button">Copy</button>
            </div>
          </div>
          <div class="transcript-text" tabindex="0"
            ><p class="transcript-placeholder">Your words, ready to use.<br />Start a recording to see your transcript here.</p
            ><span class="transcript-final"></span
            ><span class="transcript-partial" aria-label="Unconfirmed words"></span
          ></div>
          <div class="cleanup-feedback">
            <button class="restore-button" type="button" hidden>Restore original</button>
            <button class="cleanup-setup-button" type="button">Add a Gemini API key in Settings</button>
            <p class="cleanup-status settings-note" role="status" aria-live="polite"></p>
          </div>
        </section>
      </section>

      <section class="settings-view" aria-label="Settings" hidden>
        <section class="settings-section" aria-label="General">
          <h2 class="section-title">General</h2>
          <label class="checkbox-field" for="launch-on-startup">
            <input
              class="launch-on-startup-checkbox"
              id="launch-on-startup"
              type="checkbox"
            />
            <span>Launch on startup</span>
          </label>
          <p class="launch-on-startup-status settings-note" role="status">
            Starts hidden in the tray when you sign in.
          </p>
          <h3 class="settings-subtitle">After recording</h3>
          <label class="checkbox-field" for="auto-copy-transcript">
            <input
              class="auto-copy-checkbox"
              id="auto-copy-transcript"
              type="checkbox"
            />
            <span>Automatically copy transcript when done</span>
          </label>
          <label class="checkbox-field" for="paste-to-target">
            <input
              class="paste-to-target-checkbox"
              id="paste-to-target"
              type="checkbox"
            />
            <span>Paste transcript into the focused app</span>
          </label>
        </section>

        <section class="settings-section" aria-label="Recording">
          <h2 class="section-title">Recording</h2>
          <label class="settings-field" for="input-device">
            <span class="settings-field-label">Microphone</span>
            <select class="input-device-select" id="input-device"></select>
          </label>
          <p class="input-device-status" role="status"></p>
          <p class="settings-note">
            If the selected microphone is missing when recording starts, the
            system default is used instead.
          </p>
          <label class="settings-field" for="max-recording-seconds">
            <span class="settings-field-label">Recording limit (seconds)</span>
            <input
              class="max-recording-seconds-input"
              id="max-recording-seconds"
              type="number"
              min="1"
              step="1"
              inputmode="numeric"
            />
          </label>
          <label class="checkbox-field live-transcript-field" for="live-transcript">
            <input
              class="live-transcript-checkbox"
              id="live-transcript"
              type="checkbox"
            />
            <span>Show live transcript</span>
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
            <span>Show unconfirmed words</span>
          </label>
        </section>

        <section class="settings-section" aria-label="Shortcuts">
          <h2 class="section-title">Shortcuts</h2>
          <div class="settings-group">
            <h3 class="settings-subtitle">Start / stop recording</h3>
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
          </div>
          <div class="settings-group">
            <h3 class="settings-subtitle">Show / hide window</h3>
            <p class="settings-note">
              Show and focus QuickText, or hide it to the tray. Recording continues
              independently. Use a different shortcut from recording.
            </p>
            <button class="window-keybind-button keybind-button" type="button">
              Set window shortcut
            </button>
            <p class="window-keybind-status" role="status"></p>
          </div>
        </section>

        <section class="settings-section" aria-label="Transcription">
          <h2 class="section-title">Transcription</h2>
          <label class="settings-field" for="transcription-provider">
            <span class="settings-field-label">Provider</span>
            <select class="provider-select" id="transcription-provider">
              <option value="" disabled>Choose a provider</option>
              <option value="soniox">Soniox</option>
              <option value="deepgram">Deepgram</option>
            </select>
          </label>
          <p class="provider-status settings-note" role="status" aria-live="polite">
            Provider changes apply to the next recording.
          </p>
          <div class="settings-group soniox-section" aria-label="Soniox settings">
            <div class="credential-settings">
              <div class="credential-status-row">
                <p class="api-key-status settings-note" role="status"></p>
                <button class="api-key-delete-button" type="button" aria-label="Remove Soniox API key">Remove</button>
              </div>
              <details class="settings-disclosure credential-editor" open>
                <summary>Set up Soniox API key</summary>
                <div class="settings-group">
                  <label class="api-key-label" for="soniox-api-key">Soniox API key</label>
                  <input class="api-key-input" id="soniox-api-key" type="password" autocomplete="off" spellcheck="false" placeholder="Paste key" />
                  <div class="api-key-actions">
                    <button class="api-key-save-button" type="button">Save key</button>
                  </div>
                </div>
              </details>
            </div>
            <div class="settings-field">
              <span class="settings-field-label">Transcription languages</span>
              <details class="language-picker">
                <summary>
                  <span class="language-picker-summary">Automatic detection</span>
                </summary>
                <div class="language-picker-panel">
                  <input
                    class="language-search-input"
                    type="search"
                    autocomplete="off"
                    placeholder="Search languages"
                    aria-label="Search transcription languages"
                  />
                  <div
                    class="language-options"
                    role="group"
                    aria-label="Preferred transcription languages"
                    ></div>
                  <button class="clear-languages-button" type="button">
                    Clear selections
                  </button>
                </div>
              </details>
            </div>
            <p class="language-status settings-note" role="status">
              Automatic detection is used when no language is selected.
            </p>
            <details class="settings-disclosure context-disclosure">
              <summary>Topic and context · Optional</summary>
              <div class="settings-group">
                <label class="settings-field" for="transcription-description">
                  <span class="settings-field-label">Topic and context</span>
                  <textarea
                    class="transcription-description-input"
                    id="transcription-description"
                    maxlength="10000"
                    rows="5"
                    placeholder="Describe the topic, project, or background"
                    aria-describedby="transcription-description-status"
                    ></textarea>
                </label>
                <button class="transcription-description-save-button" type="button">
                  Save context
                </button>
                <p
                  class="transcription-description-status settings-note"
                  id="transcription-description-status"
                  role="status"
                >
                  Applied to the next recording. Leave empty to send no description.
                </p>
              </div>
            </details>
          </div>
          <div class="settings-group deepgram-section" aria-label="Deepgram settings">
            <div class="credential-settings">
              <div class="credential-status-row">
                <p class="deepgram-api-key-status settings-note" role="status"></p>
                <button class="deepgram-api-key-delete-button" type="button" aria-label="Remove Deepgram API key">Remove</button>
              </div>
              <details class="settings-disclosure credential-editor" open>
                <summary>Set up Deepgram API key</summary>
                <div class="settings-group">
                  <label class="api-key-label" for="deepgram-api-key">Deepgram API key</label>
                  <input class="deepgram-api-key-input" id="deepgram-api-key" type="password" autocomplete="off" spellcheck="false" placeholder="Paste key" />
                  <div class="api-key-actions">
                    <button class="deepgram-api-key-save-button" type="button">Save key</button>
                  </div>
                </div>
              </details>
            </div>
            <p class="settings-note">
              Deepgram uses Nova 3 with English transcription in this release.
              Language options will be added later.
            </p>
          </div>
          <details class="settings-disclosure terms-disclosure">
            <summary>Terms and phrases · Optional</summary>
            <div class="settings-group">
              <label class="settings-field" for="transcription-terms">
                <span class="settings-field-label">Terms and phrases</span>
                <textarea
                  class="transcription-terms-input"
                  id="transcription-terms"
                  maxlength="10000"
                  rows="5"
                  placeholder="Soniox&#10;Hyprland&#10;QuickText"
                  aria-describedby="transcription-terms-status"
                  ></textarea>
              </label>
              <button class="transcription-terms-save-button" type="button">
                Save terms
              </button>
              <p
                class="transcription-terms-status settings-note"
                id="transcription-terms-status"
                role="status"
              >
                Enter one term or phrase per line. Blank lines are ignored.
              </p>
            </div>
          </details>
        </section>

        <section class="settings-section cleanup-section">
          <h2 class="section-title">Transcript cleanup</h2>
          <p class="settings-note">Rewrite transcripts with Clean &amp; Copy, powered by Google Gemini.</p>
          <div class="credential-settings">
            <div class="credential-status-row">
              <p class="cleanup-api-key-status settings-note" id="cleanup-api-key-status" role="status"></p>
              <button class="cleanup-api-key-delete-button" type="button" aria-label="Remove Gemini API key">Remove</button>
            </div>
            <details class="settings-disclosure credential-editor" open>
              <summary>Set up Gemini API key</summary>
              <div class="settings-group">
                <label class="api-key-label" for="gemini-cleanup-api-key">Gemini API key</label>
                <input class="cleanup-api-key-input" id="gemini-cleanup-api-key" type="password" autocomplete="off" spellcheck="false" maxlength="4096" placeholder="Paste key" aria-describedby="cleanup-api-key-status" />
                <div class="api-key-actions">
                  <button class="cleanup-api-key-save-button" type="button">Save key</button>
                </div>
              </div>
            </details>
          </div>
          <a class="cleanup-key-link" href="https://aistudio.google.com/apikey" target="_blank" rel="noopener noreferrer">Get an API key in Google AI Studio</a>
          <p class="settings-note">Clean &amp; Copy sends transcript text to Google for rewriting.
          Free-tier content may be used to improve Google's products. Billing setup is optional.</p>
        </section>

        <section class="settings-section">
          <h2 class="section-title">About</h2>
          <p class="settings-note">
            Closing the window keeps QuickText running in the tray. Use Quit in the tray menu to exit.
          </p>
          <dl class="build-info" aria-label="Application build information">
            <div class="build-info-row">
              <dt>Version</dt>
              <dd class="app-version">Loading...</dd>
            </div>
          </dl>
          <details class="settings-disclosure">
            <summary>Build details</summary>
            <dl class="build-info">
              <div class="build-info-row">
                <dt>Build</dt>
                <dd class="build-id">Loading...</dd>
              </div>
              <div class="build-info-row">
                <dt>Commit</dt>
                <dd class="source-revision">Loading...</dd>
              </div>
            </dl>
          </details>
          <button class="copy-build-info-button" type="button">
            Copy build information
          </button>
          <p class="build-info-status settings-note" role="status" aria-live="polite"></p>
        </section>
      </section>
    </main>

    <footer class="bottom-bar">
      <div class="bottom-bar-meta">
        <span class="shortcut-hint"></span>
        <span class="resident-label">Tray / background</span>
      </div>
      <p class="bottom-status" role="status"></p>
    </footer>
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
  const providerLabel =
    root.querySelector<HTMLSpanElement>(".provider-label");
  const deviceNotice =
    root.querySelector<HTMLParagraphElement>(".device-notice");
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
  const cleanupKeyLink = root.querySelector<HTMLAnchorElement>(".cleanup-key-link");
  const cleanButton = root.querySelector<HTMLButtonElement>(".clean-button");
  const restoreButton = root.querySelector<HTMLButtonElement>(".restore-button");
  const cleanupSetupButton = root.querySelector<HTMLButtonElement>(".cleanup-setup-button");
  const cleanupFeedback = root.querySelector<HTMLDivElement>(".cleanup-feedback");
  const cleanupStatus = root.querySelector<HTMLParagraphElement>(".cleanup-status");
  const cleanupApiKeyInput = root.querySelector<HTMLInputElement>(".cleanup-api-key-input");
  const cleanupApiKeySaveButton = root.querySelector<HTMLButtonElement>(".cleanup-api-key-save-button");
  const cleanupApiKeyDeleteButton = root.querySelector<HTMLButtonElement>(".cleanup-api-key-delete-button");
  const cleanupApiKeyStatus = root.querySelector<HTMLParagraphElement>(".cleanup-api-key-status");
  const copyButton = root.querySelector<HTMLButtonElement>(".copy-button");
  const keybindButton =
    root.querySelector<HTMLButtonElement>(".keybind-button:not(.window-keybind-button)");
  const windowKeybindStatus = root.querySelector<HTMLParagraphElement>(".window-keybind-status");
  const windowKeybindButton = root.querySelector<HTMLButtonElement>(".window-keybind-button");
  const keybindStatus =
    root.querySelector<HTMLParagraphElement>(".keybind-status");
  const shortcutFocusOnStartCheckbox = root.querySelector<HTMLInputElement>(
    ".shortcut-focus-on-start-checkbox",
  );
  const shortcutHideOnStopCheckbox = root.querySelector<HTMLInputElement>(
    ".shortcut-hide-on-stop-checkbox",
  );
  const providerSelect =
    root.querySelector<HTMLSelectElement>(".provider-select");
  const providerStatus = root.querySelector<HTMLParagraphElement>(".provider-status");
  const sonioxSection = root.querySelector<HTMLElement>(".soniox-section");
  const apiKeyInput = root.querySelector<HTMLInputElement>(".api-key-input");
  const apiKeySaveButton = root.querySelector<HTMLButtonElement>(
    ".api-key-save-button",
  );
  const apiKeyDeleteButton = root.querySelector<HTMLButtonElement>(
    ".api-key-delete-button",
  );
  const apiKeyStatus =
    root.querySelector<HTMLParagraphElement>(".api-key-status");
  const deepgramSection =
    root.querySelector<HTMLElement>(".deepgram-section");
  const deepgramApiKeyInput = root.querySelector<HTMLInputElement>(
    ".deepgram-api-key-input",
  );
  const deepgramApiKeySaveButton = root.querySelector<HTMLButtonElement>(
    ".deepgram-api-key-save-button",
  );
  const deepgramApiKeyDeleteButton = root.querySelector<HTMLButtonElement>(
    ".deepgram-api-key-delete-button",
  );
  const deepgramApiKeyStatus = root.querySelector<HTMLParagraphElement>(
    ".deepgram-api-key-status",
  );
  const transcriptionDescriptionInput =
    root.querySelector<HTMLTextAreaElement>(
      ".transcription-description-input",
    );
  const transcriptionDescriptionSaveButton =
    root.querySelector<HTMLButtonElement>(
      ".transcription-description-save-button",
    );
  const transcriptionDescriptionStatus =
    root.querySelector<HTMLParagraphElement>(
      ".transcription-description-status",
    );
  const transcriptionTermsInput = root.querySelector<HTMLTextAreaElement>(
    ".transcription-terms-input",
  );
  const transcriptionTermsSaveButton =
    root.querySelector<HTMLButtonElement>(
      ".transcription-terms-save-button",
    );
  const transcriptionTermsStatus = root.querySelector<HTMLParagraphElement>(
    ".transcription-terms-status",
  );
  const languagePicker =
    root.querySelector<HTMLDetailsElement>(".language-picker");
  const languagePickerSummary = root.querySelector<HTMLSpanElement>(
    ".language-picker-summary",
  );
  const languageSearchInput = root.querySelector<HTMLInputElement>(
    ".language-search-input",
  );
  const languageOptions =
    root.querySelector<HTMLDivElement>(".language-options");
  const clearLanguagesButton = root.querySelector<HTMLButtonElement>(
    ".clear-languages-button",
  );
  const languageStatus =
    root.querySelector<HTMLParagraphElement>(".language-status");
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
  const pasteToTargetCheckbox = root.querySelector<HTMLInputElement>(
    ".paste-to-target-checkbox",
  );
  const launchOnStartupCheckbox = root.querySelector<HTMLInputElement>(
    ".launch-on-startup-checkbox",
  );
  const launchOnStartupStatus = root.querySelector<HTMLParagraphElement>(
    ".launch-on-startup-status",
  );
  const appVersion = root.querySelector<HTMLSpanElement>(".app-version");
  const buildId = root.querySelector<HTMLSpanElement>(".build-id");
  const sourceRevision = root.querySelector<HTMLSpanElement>(
    ".source-revision",
  );
  const copyBuildInfoButton = root.querySelector<HTMLButtonElement>(
    ".copy-build-info-button",
  );
  const buildInfoStatus = root.querySelector<HTMLParagraphElement>(
    ".build-info-status",
  );
  const bottomStatus =
    root.querySelector<HTMLParagraphElement>(".bottom-status");
  const shortcutHint = root.querySelector<HTMLSpanElement>(".shortcut-hint");
  const audioLevelMeter = root.querySelector<HTMLElement>(".activity-bars");
  const audioLevelBars = Array.from(root.querySelectorAll<HTMLElement>(".activity-bars i"));

  if (
    !statusChip ||
    !viewToggleButton ||
    !captureView ||
    !settingsView ||
    !recordButton ||
    !providerLabel ||
    !deviceNotice ||
    !recordingTimer ||
    !activityIndicator ||
    !transcriptText ||
    !transcriptPlaceholder ||
    !transcriptFinal ||
    !transcriptPartial ||
    !cleanupKeyLink ||
    !cleanButton ||
    !restoreButton ||
    !cleanupSetupButton ||
    !cleanupFeedback ||
    !cleanupStatus ||
    !cleanupApiKeyInput ||
    !cleanupApiKeySaveButton ||
    !cleanupApiKeyDeleteButton ||
    !cleanupApiKeyStatus ||
    !copyButton ||
    !keybindButton ||
    !keybindStatus ||
    !windowKeybindButton ||
    !windowKeybindStatus ||
    !shortcutFocusOnStartCheckbox ||
    !shortcutHideOnStopCheckbox ||
    !providerSelect ||
    !providerStatus ||
    !sonioxSection ||
    !apiKeyInput ||
    !apiKeySaveButton ||
    !apiKeyDeleteButton ||
    !apiKeyStatus ||
    !deepgramSection ||
    !deepgramApiKeyInput ||
    !deepgramApiKeySaveButton ||
    !deepgramApiKeyDeleteButton ||
    !deepgramApiKeyStatus ||
    !transcriptionDescriptionInput ||
    !transcriptionDescriptionSaveButton ||
    !transcriptionDescriptionStatus ||
    !transcriptionTermsInput ||
    !transcriptionTermsSaveButton ||
    !transcriptionTermsStatus ||
    !languagePicker ||
    !languagePickerSummary ||
    !languageSearchInput ||
    !languageOptions ||
    !clearLanguagesButton ||
    !languageStatus ||
    !inputDeviceSelect ||
    !inputDeviceStatus ||
    !maxRecordingSecondsInput ||
    !autoCopyCheckbox ||
    !liveTranscriptField ||
    !liveTranscriptCheckbox ||
    !showPartialField ||
    !showPartialCheckbox ||
    !pasteToTargetCheckbox ||
    !launchOnStartupCheckbox ||
    !launchOnStartupStatus ||
    !appVersion ||
    !buildId ||
    !sourceRevision ||
    !copyBuildInfoButton ||
    !buildInfoStatus ||
    !bottomStatus ||
    !shortcutHint ||
    !audioLevelMeter ||
    !audioLevelBars.length
  ) {
    throw new Error("App controls were not found");
  }

  return {
    audioLevelMeter,
    audioLevelBars,
    shortcutHint,
    statusChip,
    viewToggleButton,
    captureView,
    settingsView,
    recordButton,
    providerLabel,
    deviceNotice,
    recordingTimer,
    activityIndicator,
    transcriptText,
    transcriptPlaceholder,
    transcriptFinal,
    transcriptPartial,
    cleanupKeyLink,
    cleanButton,
    restoreButton,
    cleanupSetupButton,
    cleanupFeedback,
    cleanupStatus,
    cleanupApiKeyInput,
    cleanupApiKeySaveButton,
    cleanupApiKeyDeleteButton,
    cleanupApiKeyStatus,
    copyButton,
    keybindButton,
    keybindStatus,
    windowKeybindButton,
    windowKeybindStatus,
    shortcutFocusOnStartCheckbox,
    shortcutHideOnStopCheckbox,
    providerSelect,
    providerStatus,
    sonioxSection,
    apiKeyInput,
    apiKeySaveButton,
    apiKeyDeleteButton,
    apiKeyStatus,
    deepgramSection,
    deepgramApiKeyInput,
    deepgramApiKeySaveButton,
    deepgramApiKeyDeleteButton,
    deepgramApiKeyStatus,
    transcriptionDescriptionInput,
    transcriptionDescriptionSaveButton,
    transcriptionDescriptionStatus,
    transcriptionTermsInput,
    transcriptionTermsSaveButton,
    transcriptionTermsStatus,
    languagePicker,
    languagePickerSummary,
    languageSearchInput,
    languageOptions,
    clearLanguagesButton,
    languageStatus,
    inputDeviceSelect,
    inputDeviceStatus,
    maxRecordingSecondsInput,
    autoCopyCheckbox,
    liveTranscriptField,
    liveTranscriptCheckbox,
    showPartialField,
    showPartialCheckbox,
    pasteToTargetCheckbox,
    launchOnStartupCheckbox,
    launchOnStartupStatus,
    appVersion,
    buildId,
    sourceRevision,
    copyBuildInfoButton,
    buildInfoStatus,
    bottomStatus,
  };
};
