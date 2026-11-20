use std::{
    fs,
    path::PathBuf,
    sync::{
        mpsc::{Receiver, RecvTimeoutError},
        Mutex,
    },
    time::Duration,
};

use flexaudio::{OutputFormat, SourceKind, Stream, StreamConfig};

use crate::{
    audio_recorder::{AudioEncoding, AudioFormat},
    settings_file::write_json,
};

const SAMPLE_RATE: u32 = 16_000;
const CHANNELS: u16 = 1;

pub struct SystemAudioPreference {
    path: Option<PathBuf>,
    enabled: Mutex<bool>,
}

impl SystemAudioPreference {
    pub fn load(path: Option<PathBuf>) -> Self {
        let enabled = path
            .as_ref()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(false);
        Self {
            path,
            enabled: Mutex::new(enabled),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.lock().map(|enabled| *enabled).unwrap_or(false)
    }

    pub fn save(&self, enabled: bool) -> Result<(), String> {
        let error = || "Could not save the system audio preference.".to_string();
        let path = self.path.as_ref().ok_or_else(error)?;
        let mut saved = self.enabled.lock().map_err(|_| error())?;
        write_json(path, &enabled).map_err(|_| error())?;
        *saved = enabled;
        Ok(())
    }
}

pub fn format() -> AudioFormat {
    AudioFormat {
        sample_rate: SAMPLE_RATE,
        channels: CHANNELS,
        encoding: AudioEncoding::I16,
    }
}

pub fn start(microphone: Option<String>) -> Result<Stream, String> {
    let mut stream = flexaudio::open(StreamConfig {
        kind: SourceKind::Mix,
        mix_mic_device_id: microphone,
        output: OutputFormat {
            sample_rate: SAMPLE_RATE,
            channels: CHANNELS,
        },
        silence_probe: false,
        ..Default::default()
    })
    .map_err(|_| capture_error())?;
    stream.start().map_err(|_| capture_error())?;
    Ok(stream)
}

pub fn run(
    mut stream: Stream,
    stop: Receiver<()>,
    mut send: impl FnMut(&[i16]),
) -> Result<(), String> {
    let result = loop {
        let stopping = match stop.recv_timeout(Duration::from_millis(10)) {
            Ok(()) => true,
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => break Ok(()),
        };
        while stream.poll_event().is_some() {}
        while let Some(chunk) = stream.poll_chunk() {
            let pcm: Vec<_> = chunk.data.into_iter().map(pcm_sample).collect();
            send(&pcm);
        }
        if stream.terminal_error().is_some() {
            break Err(capture_error());
        }
        if stopping {
            break Ok(());
        }
    };
    stream.stop();
    result
}

fn pcm_sample(sample: f32) -> i16 {
    (sample * 32_768.0).round() as i16
}

fn capture_error() -> String {
    let requirements = if cfg!(target_os = "macos") {
        "Check System Settings → Privacy & Security for audio capture permission (macOS 14.4+ required)."
    } else if cfg!(target_os = "linux") {
        "Check that PipeWire is running and a default output device is available."
    } else {
        "Check Windows microphone permissions and your default playback device."
    };
    format!("Could not capture microphone and system audio. {requirements} You can turn off Include system audio to record just the microphone.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_audio_stays_off_until_explicitly_enabled_and_saved() {
        let path = std::env::temp_dir().join(format!(
            "quicktext-system-audio-test-{}.json",
            std::process::id()
        ));
        fs::write(&path, b"invalid JSON").unwrap();
        let preference = SystemAudioPreference::load(Some(path.clone()));
        assert!(!preference.enabled());
        preference.save(true).unwrap();

        let restored = SystemAudioPreference::load(Some(path.clone()));
        assert!(restored.enabled());
        restored.save(false).unwrap();
        assert!(!SystemAudioPreference::load(Some(path.clone())).enabled());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn converts_normalized_samples_to_saturating_pcm16() {
        assert_eq!(pcm_sample(-1.5), i16::MIN);
        assert_eq!(pcm_sample(-0.5), -16_384);
        assert_eq!(pcm_sample(0.0), 0);
        assert_eq!(pcm_sample(0.5), 16_384);
        assert_eq!(pcm_sample(1.5), i16::MAX);
        assert_eq!(pcm_sample(f32::NAN), 0);
    }
}
