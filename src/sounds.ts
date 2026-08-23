export type ToneStep = {
  frequencyHz: number;
  startMs: number;
  durationMs: number;
};

export type SoundClip = {
  id: string;
  label: string;
  tones: ToneStep[];
};

export const soundClips: SoundClip[] = [
  {
    id: "subtle-click",
    label: "Subtle click",
    tones: [{ frequencyHz: 880, startMs: 0, durationMs: 70 }],
  },
  {
    id: "soft-chime",
    label: "Soft chime",
    tones: [
      { frequencyHz: 660, startMs: 0, durationMs: 90 },
      { frequencyHz: 990, startMs: 110, durationMs: 150 },
    ],
  },
  {
    id: "double-beep",
    label: "Double beep",
    tones: [
      { frequencyHz: 440, startMs: 0, durationMs: 80 },
      { frequencyHz: 440, startMs: 180, durationMs: 80 },
    ],
  },
];

export const defaultStartSoundClipId = "subtle-click";
export const defaultStopSoundClipId = "soft-chime";

export const getSoundClip = (clipId: string): SoundClip => {
  return soundClips.find((clip) => clip.id === clipId) ?? soundClips[0];
};

const toneGain = 0.08;

let audioContext: AudioContext | null | undefined;

const getAudioContext = (): AudioContext | null => {
  if (audioContext !== undefined) {
    return audioContext;
  }

  try {
    audioContext = new AudioContext();
  } catch {
    audioContext = null;
  }

  return audioContext;
};

export const playSoundClip = (clipId: string): void => {
  const context = getAudioContext();

  if (!context) {
    return;
  }

  if (context.state === "suspended") {
    void context.resume().catch(() => {});
  }

  const clip = getSoundClip(clipId);

  for (const tone of clip.tones) {
    try {
      const oscillator = context.createOscillator();
      const gain = context.createGain();
      const startTime = context.currentTime + tone.startMs / 1000;
      const endTime = startTime + tone.durationMs / 1000;

      oscillator.type = "sine";
      oscillator.frequency.setValueAtTime(tone.frequencyHz, startTime);
      gain.gain.setValueAtTime(0.0001, startTime);
      gain.gain.exponentialRampToValueAtTime(toneGain, startTime + 0.012);
      gain.gain.exponentialRampToValueAtTime(0.0001, endTime);

      oscillator.connect(gain);
      gain.connect(context.destination);
      oscillator.start(startTime);
      oscillator.stop(endTime + 0.02);
    } catch {
      return;
    }
  }
};
