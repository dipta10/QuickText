import type { SpeechView } from "./speech-view";
import { listSpeechVoices, type SpeechVoice } from "./tauri";

const message = (error: unknown) => error instanceof Error ? error.message : String(error);

export const connectSpeechVoicePicker = (
  view: SpeechView,
  hasApiKey: () => boolean,
  onEdit: () => void,
) => {
  let voices: SpeechVoice[] = [];
  let loaded = false;
  let loading = false;
  let requestRevision = 0;

  const render = () => {
    const selected = voices.find((voice) => voice.id === view.voice.value);
    view.voiceDescription.textContent = selected?.description ||
      (loaded && view.voice.value ? "Saved or custom voice." : "");
    view.voice.setAttribute("aria-busy", String(loading));
    view.refreshVoices.disabled = loading || !hasApiKey();
    view.refreshVoices.textContent = loading ? "Loading…" : "Refresh voices";
  };
  const setValue = (id: string) => {
    for (const option of Array.from(view.voice.options)) {
      if (option.value !== id && !voices.some((voice) => voice.id === option.value)) option.remove();
    }
    if (!Array.from(view.voice.options).some((option) => option.value === id)) {
      view.voice.add(new Option(id, id));
    }
    view.voice.value = id;
    if (!voices.some((voice) => voice.id === id)) view.customVoice.value = id;
    render();
  };
  const load = async (force = false) => {
    if (!hasApiKey()) {
      requestRevision += 1;
      loading = false;
      view.voiceStatus.textContent = "Add a Soniox API key to load available voices.";
      render();
      return;
    }
    if ((loading || loaded) && !force) return;
    const revision = ++requestRevision;
    loading = true;
    view.voiceStatus.textContent = "Loading available voices…";
    render();
    try {
      const available = await listSpeechVoices();
      if (revision !== requestRevision) return;
      // Read at completion, so a refresh never replaces a newer user selection.
      const selected = view.voice.value;
      voices = available;
      loaded = true;
      view.voice.replaceChildren(...voices.map((voice) => {
        const gender = ({ male: "Male", female: "Female", neutral: "Neutral" } as Record<string, string>)[voice.gender];
        const option = new Option(gender ? `${voice.id} · ${gender}` : voice.id, voice.id);
        option.title = voice.description;
        return option;
      }));
      setValue(selected);
      view.voiceStatus.textContent = `${voices.length} ${voices.length === 1 ? "voice" : "voices"} available. Changes apply to your next reading when saved.`;
    } catch (error) {
      if (revision === requestRevision) view.voiceStatus.textContent = message(error);
    } finally {
      if (revision === requestRevision) { loading = false; render(); }
    }
  };
  view.voice.addEventListener("change", () => { render(); onEdit(); });
  view.customVoice.addEventListener("input", () => {
    const id = view.customVoice.value.trim();
    if (!id) return;
    setValue(id);
    onEdit();
  });
  view.refreshVoices.addEventListener("click", () => { void load(true); });
  return { render, setValue, load };
};
