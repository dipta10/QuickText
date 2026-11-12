import { closeCredentialEditor, renderCredentialEditor } from "./credential-editor";
import type { AppView } from "./app-view";
import type { AppState } from "./app-state";
import { canChangeProvider, showSettings, setProviderApiKeyPresence, setProviderApiKeyStatus } from "./app-state";
import { saveProviderApiKey, deleteProviderApiKey, getSpeechState, getSpeechSettings, onSpeechStateChanged, readClipboardText, setSpeechSettings, toggleSpeech, stopSpeech, type SpeechSettings, type SpeechSnapshot, type SupportedLanguage } from "./tauri";
import { acceptSpeechSnapshot, emptySpeechSnapshot, speechIsActive, speechLabel } from "./speech-state";

const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);
export const connectSpeechControls = (app: AppView, getState: () => AppState, updateState: (state: AppState) => void) => {
  const view = app.speech;
  let snapshot = emptySpeechSnapshot();
  let settings: SpeechSettings = { voice: "Adrian", language: "en", speed: 1 };
  let settingsReady = false;
  let saving = false;
  let pending = false;
  let feedback = "";
  let settingsEdited = false;
  let credentialPending = false;
  let languages: SupportedLanguage[] = [];
  const render = () => {
    const state = getState();
    const active = speechIsActive(snapshot);
    const count = Array.from(view.input.value).length;
    view.panel.hidden = state.activeView !== "speech";
    view.count.textContent = `${count.toLocaleString()} / 5,000`;
    view.input.readOnly = active;
    view.paste.disabled = active;
    view.clear.disabled = active || !count;
    view.play.disabled = pending || (!active && (!count || !canChangeProvider(state)));
    view.playLabel.textContent = active ? "Stop reading" : view.input.selectionEnd > view.input.selectionStart ? "Read selection" : snapshot.status === "completed" ? "Read again" : "Read aloud";
    view.playIcon.textContent = active ? "■" : "▶";
    view.play.setAttribute("aria-label", view.playLabel.textContent);
    view.player.dataset.status = snapshot.status;
    view.status.textContent = speechLabel(snapshot);
    view.feedback.textContent = feedback || snapshot.error || (active ? "Streaming to your speakers. Stop at any time." : "Audio plays through your system output.");
    view.feedback.dataset.error = String(Boolean(snapshot.error || feedback));
    view.summary.textContent = `${settings.voice} · ${settings.speed.toFixed(2).replace(/0$/, "")}×`;
    view.shortcutHint.textContent = state.selectedSpeechShortcut || "a shortcut from Settings";
    view.setup.hidden = state.providerApiKeyConfigured.soniox;
    renderCredentialEditor(view.keyInput, state.providerApiKeyConfigured.soniox);
    view.keyInput.disabled = credentialPending || !canChangeProvider(state);
    view.keySave.disabled = credentialPending || !canChangeProvider(state);
    view.keyDelete.disabled = credentialPending || !canChangeProvider(state) || !state.providerApiKeyConfigured.soniox;
    view.keyStatus.textContent = state.providerApiKeyStatus.soniox || (state.providerApiKeyConfigured.soniox ? "API key saved. Shared with Soniox dictation." : "Add a Soniox key to start reading aloud.");
    view.save.disabled = !settingsReady || saving || !settingsEdited;
    view.voice.disabled = !settingsReady || saving;
    view.language.disabled = !settingsReady || saving;
    view.speed.disabled = !settingsReady || saving;
    const indicator = app.speechTab.querySelector<HTMLElement>(".speech-tab-indicator");
    if (indicator) indicator.hidden = !active;
    if (state.activeView === "speech") {
      app.shortcutHint.textContent = state.selectedSpeechShortcut || "Set a reading shortcut in Settings";
      app.bottomStatus.textContent = snapshot.error || (active ? "Reading continues when the window is hidden." : "Text is sent only when you start reading. Up to two minutes per reading.");
    }
  };
  const accept = (next: SpeechSnapshot) => {
    const accepted = acceptSpeechSnapshot(snapshot, next);
    if (accepted !== snapshot) {
      const beganPlayback = accepted.status === "playing" && snapshot.status !== "playing";
      const newError = accepted.status === "error" && canChangeProvider(getState());
      snapshot = accepted;
      feedback = "";
      if ((beganPlayback || newError) && getState().activeView !== "settings") updateState({ ...getState(), activeView: "speech" });
    }
    render();
  };
  view.input.addEventListener("select", render);
  view.input.addEventListener("input", () => { feedback = ""; render(); });
  view.clear.addEventListener("click", () => { view.input.value = ""; feedback = ""; view.input.focus(); render(); });
  view.paste.addEventListener("click", () => {
    void readClipboardText().then((text) => {
      if (speechIsActive(snapshot)) return;
      view.input.value = Array.from(text).slice(0, 5000).join("");
      feedback = Array.from(text).length > 5000 ? "Pasted the first 5,000 characters. Split longer text into shorter passages." : "";
      view.input.focus();
      render();
    }).catch((error: unknown) => { feedback = errorMessage(error); render(); });
  });
  const play = async () => {
    if (pending) return;
    pending = true;
    feedback = "";
    render();
    try {
      const text = view.input.selectionEnd > view.input.selectionStart ? view.input.value.slice(view.input.selectionStart, view.input.selectionEnd) : view.input.value;
      accept(speechIsActive(snapshot) ? await stopSpeech() : await toggleSpeech(text));
    }
    catch (error) { feedback = errorMessage(error); }
    finally { pending = false; render(); }
  };
  view.play.addEventListener("click", () => { void play(); });
  view.input.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) { event.preventDefault(); void play(); }
  });
  view.setup.addEventListener("click", () => {
    updateState(showSettings(getState()));
    const editor = view.keyInput.closest<HTMLDetailsElement>("details");
    if (editor) editor.open = true;
    view.keyInput.focus();
    view.keyInput.scrollIntoView?.({ block: "center" });
  });
  const credentialAction = async (remove: boolean) => {
    if (credentialPending || !canChangeProvider(getState())) return;
    credentialPending = true;
    render();
    try {
      if (remove) await deleteProviderApiKey("soniox");
      else {
        const configured = await saveProviderApiKey("soniox", view.keyInput.value);
        if (!configured) throw new Error("The API key could not be read after saving. Try again.");
      }
      view.keyInput.value = "";
      if (!remove) closeCredentialEditor(view.keyInput);
      updateState(setProviderApiKeyPresence(getState(), "soniox", !remove, remove ? "API key deleted." : "API key saved."));
    } catch (error) { updateState(setProviderApiKeyStatus(getState(), "soniox", errorMessage(error))); }
    finally { credentialPending = false; render(); }
  };
  view.keySave.addEventListener("click", () => { void credentialAction(false); });
  view.keyDelete.addEventListener("click", () => { void credentialAction(true); });
  const edit = () => { settingsEdited = true; view.speedValue.textContent = `${Number(view.speed.value).toFixed(2).replace(/0$/, "")}×`; render(); };
  view.voice.addEventListener("input", edit);
  view.language.addEventListener("change", edit);
  view.speed.addEventListener("input", edit);
  view.save.addEventListener("click", () => {
    if (!settingsReady || saving) return;
    saving = true;
    view.settingsStatus.textContent = "Saving preferences…";
    render();
    void setSpeechSettings({ voice: view.voice.value.trim(), language: view.language.value, speed: Number(view.speed.value) }).then((saved) => {
      settings = saved;
      view.voice.value = saved.voice;
      settingsEdited = false;
      view.settingsStatus.textContent = "Voice preferences saved for your next reading.";
    }).catch((error: unknown) => { view.settingsStatus.textContent = errorMessage(error); }).finally(() => { saving = false; render(); });
  });
  const setLanguages = (available: SupportedLanguage[]) => {
    languages = available;
    view.language.replaceChildren(...languages.map((language) => new Option(language.name, language.code)));
    view.language.value = settings.language;
  };
  // Subscribe before querying to avoid missing playback while the webview loads.
  void (async () => {
    try {
      await onSpeechStateChanged(accept);
      accept(await getSpeechState());
    } catch { /* The static preview remains usable without native playback. */ }
  })();
  void getSpeechSettings().then((saved) => {
    settings = saved;
    settingsReady = true;
    view.voice.value = saved.voice;
    view.speed.value = String(saved.speed);
    view.speedValue.textContent = `${saved.speed.toFixed(2).replace(/0$/, "")}×`;
    if (languages.length) view.language.value = saved.language;
    render();
  }).catch((error: unknown) => { view.settingsStatus.textContent = errorMessage(error); });
  return { render, setLanguages };
};
