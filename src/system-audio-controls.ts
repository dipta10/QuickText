import { canChangeProvider, type AppState } from "./app-state";
import type { AppView } from "./app-view";
import { getIncludeSystemAudio, setIncludeSystemAudio } from "./tauri";

export const connectSystemAudioControls = (view: AppView, getState: () => AppState) => {
  let enabled: boolean | null = null;
  let saving = false;

  const render = () => {
    view.includeSystemAudioCheckbox.checked = enabled === true;
    view.includeSystemAudioCheckbox.disabled =
      enabled === null || saving || !canChangeProvider(getState());
    view.audioLevelLabel.textContent = enabled ? "Mic + system audio" : "Input level";
  };

  const showError = (message: string) => {
    view.systemAudioStatus.textContent = message;
    view.systemAudioStatus.hidden = message === "";
  };

  view.includeSystemAudioCheckbox.addEventListener("change", async () => {
    const requested = view.includeSystemAudioCheckbox.checked;
    saving = true;
    showError("");
    render();
    try {
      await setIncludeSystemAudio(requested);
      enabled = requested;
    } catch {
      showError("Could not save the system audio preference.");
    } finally {
      saving = false;
      render();
    }
  });

  void getIncludeSystemAudio().then((saved) => {
    enabled = saved;
    render();
  }).catch(() => {
    showError("Could not load the system audio preference.");
  });

  return render;
};
