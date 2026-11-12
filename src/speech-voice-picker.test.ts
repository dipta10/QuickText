// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import { createAppView } from "./app-view";
import { connectSpeechVoicePicker } from "./speech-voice-picker";
import { listSpeechVoices, type SpeechVoice } from "./tauri";
vi.mock("./tauri");
const catalog: SpeechVoice[] = [
  {id:"Adrian",description:"Measured delivery.",gender:"male"},
  {id:"Maya",description:"Warm and clear.",gender:"female"},
];
let view: ReturnType<typeof createAppView>["speech"];
let hasKey: boolean;
let changed = vi.fn<() => void>();
let picker: ReturnType<typeof connectSpeechVoicePicker>;
beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML='<div id="app"></div>';
  view=createAppView(document.querySelector("#app")!).speech;
  hasKey=true;changed=vi.fn<() => void>();
  picker=connectSpeechVoicePicker(view,()=>hasKey,changed);
  vi.mocked(listSpeechVoices).mockResolvedValue(catalog);
});
describe("speech voice picker", () => {
  it("lists voices with gender and the selected voice description", async () => {
    await picker.load();
    expect(Array.from(view.voice.options).map(o=>o.textContent)).toEqual(["Adrian · Male","Maya · Female"]);
    view.voice.value="Maya";view.voice.dispatchEvent(new Event("change"));
    expect(view.voiceDescription.textContent).toBe("Warm and clear.");
    expect(changed).toHaveBeenCalledOnce();
    await picker.load();
    expect(listSpeechVoices).toHaveBeenCalledOnce();
  });
  it("restores a saved choice before and after the catalog arrives", async () => {
    picker.setValue("Maya");
    await picker.load();
    expect(view.voice.value).toBe("Maya");
    expect(view.voiceDescription.textContent).toBe("Warm and clear.");
    expect(changed).not.toHaveBeenCalled();
  });
  it("retains saved custom voices and permits entering a cloned voice ID", async () => {
    picker.setValue("saved-clone-id");
    await picker.load();
    expect(view.voice.value).toBe("saved-clone-id");
    view.customVoice.value="another-clone-id";view.customVoice.dispatchEvent(new Event("input"));
    expect(view.voice.value).toBe("another-clone-id");
    expect(changed).toHaveBeenCalledOnce();
    view.customVoice.value="final-clone-id";view.customVoice.dispatchEvent(new Event("input"));
    expect(Array.from(view.voice.options).map(option=>option.value)).toEqual(["Adrian","Maya","final-clone-id"]);
  });
  it("preserves a newer choice made during a refresh", async () => {
    await picker.load();
    let resolve!: (voices:SpeechVoice[])=>void;
    vi.mocked(listSpeechVoices).mockReturnValue(new Promise(r=>{resolve=r;}));
    const refresh=picker.load(true);
    view.voice.value="Maya";view.voice.dispatchEvent(new Event("change"));
    resolve(catalog);await refresh;
    expect(view.voice.value).toBe("Maya");
  });
  it("keeps the previous list and choice when refreshing fails", async () => {
    await picker.load();picker.setValue("Maya");
    vi.mocked(listSpeechVoices).mockRejectedValue(new Error("Check your connection."));
    await picker.load(true);
    expect(view.voice.value).toBe("Maya");
    expect(view.voice.options.length).toBe(2);
    expect(view.voiceStatus.textContent).toBe("Check your connection.");
    expect(view.refreshVoices.disabled).toBe(false);
  });
  it("does not request voices without a saved key", async () => {
    hasKey=false;await picker.load();
    expect(listSpeechVoices).not.toHaveBeenCalled();
    expect(view.voiceStatus.textContent).toContain("Add a Soniox API key");
    expect(view.refreshVoices.disabled).toBe(true);
  });
  it("ignores responses from a removed or replaced credential", async () => {
    let resolve!: (voices:SpeechVoice[])=>void;
    vi.mocked(listSpeechVoices).mockReturnValueOnce(new Promise(r=>{resolve=r;}));
    const old=picker.load();
    hasKey=false;await picker.load(true);
    hasKey=true;
    vi.mocked(listSpeechVoices).mockResolvedValue([{id:"New voice",description:"Current catalog.",gender:"neutral"}]);
    await picker.load(true);
    resolve(catalog);await old;
    expect(Array.from(view.voice.options).map(o=>o.value)).toContain("New voice");
    expect(Array.from(view.voice.options).map(o=>o.value)).not.toContain("Maya");
    expect(view.voiceStatus.textContent).toContain("1 voice available");
  });
});
