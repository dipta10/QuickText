import type { AppState } from "./app-state";
import {
  canChangeProvider, canCleanTranscript, setCleanupCredentials,
  setCleanupCredentialStatus, setStatus, showSettings,
} from "./app-state";
import type { AppView } from "./app-view";
import {
  cleanAndCopy, deleteCleanupApiKey, hasCleanupApiKey, restoreOriginalTranscript,
  saveCleanupApiKey, openCleanupKeySetup,
} from "./tauri";

const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);

export const renderCleanupControls = (view: AppView, state: AppState) => {
  view.cleanButton.disabled = !canCleanTranscript(state);
  view.cleanButton.textContent = state.cleanup.running ? "Cleaning…" : "Clean & Copy";
  view.restoreButton.hidden = !state.cleanup.cleaned;
  view.restoreButton.disabled = state.cleanup.running || state.cleanupActionPending;
  view.cleanupSetupButton.hidden = state.cleanupApiKeyConfigured;
  view.cleanupStatus.textContent = state.cleanup.message;
  const disabled = !canChangeProvider(state) || state.cleanup.running || state.cleanupCredentialBusy || state.cleanupActionPending;
  view.cleanupApiKeyInput.disabled = disabled;
  view.cleanupApiKeySaveButton.disabled = disabled;
  view.cleanupApiKeyDeleteButton.disabled = disabled || !state.cleanupApiKeyConfigured;
  view.cleanupApiKeyStatus.textContent = state.cleanupApiKeyStatus ||
    (state.cleanupApiKeyConfigured ? "API key saved." : "No API key saved.");
};

export const connectCleanupControls = (
  view: AppView,
  getState: () => AppState,
  updateState: (state: AppState) => void,
) => {
  let credentialRevision = 0;
  const credentialAction = async (remove: boolean) => {
    if (getState().cleanupCredentialBusy || !canChangeProvider(getState()) || getState().cleanup.running) return;
    credentialRevision += 1;
    updateState({ ...getState(), cleanupCredentialBusy: true });
    try {
      if (remove) await deleteCleanupApiKey();
      else await saveCleanupApiKey(view.cleanupApiKeyInput.value);
      view.cleanupApiKeyInput.value = "";
      updateState(setCleanupCredentials(getState(), !remove,
        remove ? "API key deleted." : "API key saved."));
    } catch (error) {
      updateState(setCleanupCredentialStatus(getState(), errorMessage(error)));
    } finally {
      updateState({ ...getState(), cleanupCredentialBusy: false });
    }
  };
  const action = async (restore: boolean) => {
    if (getState().cleanupActionPending || (!restore && !canCleanTranscript(getState()))) return;
    updateState({ ...getState(), cleanupActionPending: true });
    const session = getState().activeSessionId;
    try {
      // Backend events carry results. Command replies never rewind newer state.
      if (restore) await restoreOriginalTranscript();
      else await cleanAndCopy();
    } catch (error) {
      if (getState().activeSessionId === session) {
        updateState(setStatus(getState(), errorMessage(error)));
      }
    } finally {
      updateState({ ...getState(), cleanupActionPending: false });
    }
  };
  view.cleanButton.addEventListener("click", () => { void action(false); });
  view.restoreButton.addEventListener("click", () => { void action(true); });
  view.cleanupApiKeySaveButton.addEventListener("click", () => { void credentialAction(false); });
  view.cleanupApiKeyDeleteButton.addEventListener("click", () => { void credentialAction(true); });
  view.cleanupSetupButton.addEventListener("click", () => {
    updateState(showSettings(getState()));
    view.cleanupApiKeyInput.focus();
    view.cleanupApiKeyInput.scrollIntoView?.({ block: "center" });
  });
  view.cleanupKeyLink.addEventListener("click", (event) => {
    event.preventDefault();
    void openCleanupKeySetup().catch((error: unknown) => {
      updateState(setCleanupCredentialStatus(getState(), errorMessage(error)));
    });
  });
  const initialRevision = credentialRevision;
  void hasCleanupApiKey().then((configured) => {
    if (credentialRevision === initialRevision) updateState(setCleanupCredentials(getState(), configured));
  }).catch((error: unknown) => {
    if (credentialRevision === initialRevision) updateState(setCleanupCredentialStatus(getState(), errorMessage(error)));
  });
};
