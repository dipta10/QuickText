// DOM factories only. Playback behavior lives in speech-controls.ts.
export const speechTemplate = `
<section class="speech-view" id="speech-panel" role="tabpanel" aria-labelledby="speech-tab" hidden>
  <div class="speech-heading">
    <div><h2>Read aloud.</h2><p>A natural voice for the words in front of you.</p></div>
    <span class="speech-provider">Soniox</span>
  </div>
  <div class="speech-composer">
    <div class="speech-editor-toolbar">
      <label for="speech-text" class="section-title">Text to read</label>
      <button class="speech-paste" type="button">Paste</button>
    </div>
    <textarea id="speech-text" class="speech-text" maxlength="5000" spellcheck="false" placeholder="An article, a message, a thought…&#10;&#10;Add text here to hear it read aloud."></textarea>
    <div class="speech-editor-meta"><span class="speech-count">0 / 5,000</span><button type="button" class="speech-clear">Clear</button></div>
  </div>
  <div class="speech-player" data-status="idle">
    <div class="speech-player-top">
      <span class="speech-status" role="status" aria-live="polite">Ready to read</span>
      <span class="speech-wave" aria-hidden="true"><i></i><i></i><i></i><i></i><i></i></span>
    </div>
    <div class="speech-player-actions">
      <button class="speech-play" type="button" disabled><span class="speech-play-icon" aria-hidden="true">▶</span><span class="speech-play-label">Read aloud</span></button>
      <span class="speech-voice-summary">Adrian · 1.0×</span>
    </div>
    <p class="speech-feedback" role="status" aria-live="polite">Audio plays through your system output.</p>
    <button type="button" class="speech-setup" hidden>Add your Soniox key in Settings</button>
  </div>
  <div class="speech-selection-note"><span aria-hidden="true">↗</span><p><strong>Read from anywhere</strong><br>Select text in another app and use <kbd class="speech-shortcut-hint">a shortcut</kbd> to read it aloud. Press again to stop.</p></div>
</section>`;

export const speechSettingsTemplate = `
<section class="settings-section speech-settings" aria-label="Text-to-speech">
  <h2 class="section-title">Text-to-speech</h2>
  <p class="settings-note">Uses your securely stored Soniox API key. Voice and language changes apply to your next reading.</p>
  <div class="speech-credentials credential-settings">
    <div class="credential-status-row"><span>Soniox API key</span><button type="button" class="speech-key-delete">Remove</button></div>
    <p class="speech-key-status settings-note" role="status"></p>
    <details class="credential-editor"><summary>Change API key</summary><div class="credential-editor-body"><label class="api-key-label" for="speech-api-key">Soniox API key</label><input id="speech-api-key" class="speech-key-input" type="password" autocomplete="off" spellcheck="false" maxlength="4096" placeholder="Paste key" /><button class="speech-key-save" type="button">Save key</button></div></details>
  </div>
  <div class="speech-settings-grid">
    <label class="field"><span>Voice</span><input class="speech-voice" value="Adrian" maxlength="50" placeholder="Adrian" autocomplete="off" aria-describedby="speech-voice-note" /></label>
    <label class="field"><span>Language</span><select class="speech-language"><option value="en">English</option></select></label>
  </div>
  <p class="settings-note" id="speech-voice-note">Use a voice name or cloned voice ID from the Soniox Console.</p>
  <label class="field speech-speed-field"><span>Reading speed <output class="speech-speed-value">1.0×</output></span><input class="speech-speed" type="range" min="0.7" max="1.3" step="0.05" value="1" /></label>
  <div class="speech-speed-scale" aria-hidden="true"><span>Slower</span><span>Natural</span><span>Faster</span></div>
  <button class="speech-save" type="button" disabled>Save voice preferences</button>
  <p class="speech-settings-status settings-note" role="status" aria-live="polite"></p>
  <div class="shortcut-field"><span class="field-label">Read selection / stop</span><div class="shortcut-actions"><button class="speech-keybind" type="button" disabled>Set shortcut</button><button class="speech-keybind-clear" type="button" disabled>Clear</button></div><p class="speech-keybind-status settings-note" role="status"></p></div>
  <div class="shortcut-field"><span class="field-label">Stop reading</span><div class="shortcut-actions"><button class="speech-stop-keybind" type="button" disabled>Set shortcut</button><button class="speech-stop-keybind-clear" type="button" disabled>Clear</button></div><p class="speech-stop-keybind-status settings-note" role="status"></p></div>
  <p class="settings-note">On Wayland, bind <code>quicktext read-selection</code> and <code>quicktext stop-speech</code> in your compositor. Reading selected text requires wl-clipboard on Wayland or xclip on X11. macOS may require Accessibility access.</p>
</section>`;

export const getSpeechView = (root: HTMLElement) => {
  const get = <T extends HTMLElement>(selector: string): T => {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Speech control missing: ${selector}`);
    return element;
  };
  return {
    panel: get<HTMLElement>(".speech-view"),
    input: get<HTMLTextAreaElement>(".speech-text"),
    paste: get<HTMLButtonElement>(".speech-paste"),
    clear: get<HTMLButtonElement>(".speech-clear"),
    count: get<HTMLSpanElement>(".speech-count"),
    player: get<HTMLDivElement>(".speech-player"),
    play: get<HTMLButtonElement>(".speech-play"),
    playLabel: get<HTMLSpanElement>(".speech-play-label"),
    playIcon: get<HTMLSpanElement>(".speech-play-icon"),
    status: get<HTMLSpanElement>(".speech-status"),
    feedback: get<HTMLParagraphElement>(".speech-feedback"),
    setup: get<HTMLButtonElement>(".speech-setup"),
    summary: get<HTMLSpanElement>(".speech-voice-summary"),
    shortcutHint: get<HTMLElement>(".speech-shortcut-hint"),
    keyInput: get<HTMLInputElement>(".speech-key-input"),
    keySave: get<HTMLButtonElement>(".speech-key-save"),
    keyDelete: get<HTMLButtonElement>(".speech-key-delete"),
    keyStatus: get<HTMLParagraphElement>(".speech-key-status"),
    voice: get<HTMLInputElement>(".speech-voice"),
    language: get<HTMLSelectElement>(".speech-language"),
    speed: get<HTMLInputElement>(".speech-speed"),
    speedValue: get<HTMLOutputElement>(".speech-speed-value"),
    save: get<HTMLButtonElement>(".speech-save"),
    settingsStatus: get<HTMLParagraphElement>(".speech-settings-status"),
    keybind: get<HTMLButtonElement>(".speech-keybind"),
    clearKeybind: get<HTMLButtonElement>(".speech-keybind-clear"),
    keybindStatus: get<HTMLParagraphElement>(".speech-keybind-status"),
    stopKeybind: get<HTMLButtonElement>(".speech-stop-keybind"),
    clearStopKeybind: get<HTMLButtonElement>(".speech-stop-keybind-clear"),
    stopKeybindStatus: get<HTMLParagraphElement>(".speech-stop-keybind-status"),
  };
};
export type SpeechView = ReturnType<typeof getSpeechView>;
