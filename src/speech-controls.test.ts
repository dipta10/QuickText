// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createAppView } from "./app-view";
import { createAppState, setProviderApiKeyPresence, type AppState } from "./app-state";
import { connectSpeechControls } from "./speech-controls";
import * as backend from "./tauri";
import type { SpeechSnapshot } from "./tauri";
vi.mock("./tauri");
const idle: SpeechSnapshot = {status:"idle",sessionId:0,revision:0,error:null};
const flush = async () => { for(let i=0;i<10;i++) await Promise.resolve(); };
let state: AppState;
let view: ReturnType<typeof createAppView>;
let event: (snapshot: SpeechSnapshot) => void;
let controls: ReturnType<typeof connectSpeechControls>;
beforeEach(async () => {
  vi.resetAllMocks();
  document.body.innerHTML = '<div id="app"></div>';
  state = createAppState("",300,false,true,false,true,true,"",false,false);
  state.activeView = "speech";
  state.providerSelectionStatus = "ready";
  state = setProviderApiKeyPresence(state, "soniox", true);
  view = createAppView(document.querySelector("#app")!);
  vi.mocked(backend.getSpeechState).mockResolvedValue(idle);
  vi.mocked(backend.getSpeechSettings).mockResolvedValue({voice:"Adrian",language:"en",speed:1});
  vi.mocked(backend.onSpeechStateChanged).mockImplementation(async (handler) => {event=handler;return () => {};});
  controls = connectSpeechControls(view, () => state, next => {state=next;controls.render();});
  controls.setLanguages([{code:"en",name:"English"},{code:"bn",name:"Bengali"}]);
  await flush();
  controls.render();
});
const write = (text: string) => { view.speech.input.value=text;view.speech.input.dispatchEvent(new Event("input")); };
describe("reading controls", () => {
  it("streams the selected passage and the same control stops while preparing", async () => {
    write("Read only this passage.");
    view.speech.input.setSelectionRange(5,9);
    vi.mocked(backend.toggleSpeech).mockResolvedValue({status:"preparing",sessionId:1,revision:1,error:null});
    view.speech.play.click(); await flush();
    expect(backend.toggleSpeech).toHaveBeenCalledWith("only");
    expect(view.speech.playLabel.textContent).toBe("Stop reading");
    expect(view.speech.input.readOnly).toBe(true);
    vi.mocked(backend.stopSpeech).mockResolvedValue({...idle, sessionId:1, revision:2});
    view.speech.play.click(); await flush();
    expect(backend.stopSpeech).toHaveBeenCalledOnce();
    expect(view.speech.input.readOnly).toBe(false);
    event({status:"playing",sessionId:1,revision:1,error:null});
    expect(view.speech.status.textContent).toBe("Ready to read");
  });
  it("keeps background reading stoppable with no text in the editor", async () => {
    state.activeView="capture";
    event({status:"preparing",sessionId:1,revision:1,error:null});
    expect(state.activeView).toBe("capture"); // Preserve the selection until capture completes.
    expect(view.speech.play.disabled).toBe(false);
    expect(view.speech.input.value).toBe("");
    event({status:"playing",sessionId:1,revision:2,error:null});
    expect(view.speech.status.textContent).toBe("Reading aloud");
    expect(state.activeView).toBe("speech");
  });
  it("preserves the draft on provider failure and displays recovery text", () => {
    write("Keep this text.");
    event({status:"error",sessionId:1,revision:1,error:"Enable text-to-speech permission for your Soniox API key."});
    expect(view.speech.input.value).toBe("Keep this text.");
    expect(view.speech.feedback.textContent).toContain("permission");
    expect(view.speech.play.disabled).toBe(false);
  });
  it("blocks starting playback during recording", () => {
    write("A passage."); state.recording="recording";controls.render();
    expect(view.speech.play.disabled).toBe(true);
    expect(backend.toggleSpeech).not.toHaveBeenCalled();
  });
  it("routes missing-key setup to shared Soniox credentials without changing the dictation provider", () => {
    state.activeProvider="deepgram";state.providerApiKeyConfigured.soniox=false;controls.render();
    view.speech.setup.click();
    expect(state.activeView).toBe("settings");
    expect(state.activeProvider).toBe("deepgram");
    expect(document.activeElement).toBe(view.speech.keyInput);
    expect(view.speech.keyInput.closest("details")!.open).toBe(true);
  });
  it("saves language, speed and voice together, and keeps the draft on save failure", async () => {
    view.speech.voice.value="Adrian";view.speech.language.value="bn";view.speech.speed.value="1.15";
    view.speech.speed.dispatchEvent(new Event("input"));
    vi.mocked(backend.setSpeechSettings).mockRejectedValue(new Error("Could not save settings."));
    view.speech.save.click(); await flush();
    expect(backend.setSpeechSettings).toHaveBeenCalledWith({voice:"Adrian",language:"bn",speed:1.15});
    expect(view.speech.settingsStatus.textContent).toBe("Could not save settings.");
    expect(view.speech.language.value).toBe("bn");
    expect(view.speech.save.disabled).toBe(false);
  });
  it("does not overwrite an active stream when a clipboard read resolves late", async () => {
    let resolve!: (text:string)=>void;
    vi.mocked(backend.readClipboardText).mockReturnValue(new Promise(r=>{resolve=r;}));
    write("Original draft.");view.speech.paste.click();
    event({status:"preparing",sessionId:1,revision:1,error:null});
    resolve("Other text");await flush();
    expect(view.speech.input.value).toBe("Original draft.");
  });
});
