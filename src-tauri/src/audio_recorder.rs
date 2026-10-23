use std::{
    mem,
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc, {Arc, Mutex},
    },
    thread::JoinHandle,
    time::Instant,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioEncoding {
    F32,
    I16,
    U16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub encoding: AudioEncoding,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCaptureStats {
    pub chunk_count: u64,
    pub sample_count: u64,
    pub byte_count: u64,
    pub elapsed_ms: u128,
}

/// cpal exposes no cross-platform stable device IDs, so the device
/// name doubles as the persisted identifier.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDeviceInfo {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDeviceList {
    pub default_label: Option<String>,
    pub devices: Vec<InputDeviceInfo>,
}

pub struct StartedRecording {
    pub recorder: AudioRecorder,
    pub fell_back_to_default: bool,
}

pub fn list_input_devices() -> InputDeviceList {
    let host = cpal::default_host();
    let default_label = host
        .default_input_device()
        .and_then(|device| device.name().ok());

    let devices = host
        .input_devices()
        .map(|devices| {
            devices
                .filter_map(|device| {
                    let name = device.name().ok()?;
                    Some(InputDeviceInfo {
                        id: name.clone(),
                        label: name,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    InputDeviceList {
        default_label,
        devices,
    }
}

pub struct AudioRecorder {
    stats: Arc<Mutex<AudioCaptureStats>>,
    level: Arc<AtomicU32>,
    command_tx: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
    started_at: Instant,
}

impl AudioRecorder {
    pub fn input_format(selected_device: Option<&str>) -> Result<AudioFormat, String> {
        let (device, _fell_back) = resolve_input_device(selected_device)?;
        let (config, sample_format) = input_config(&device)?;

        Ok(AudioFormat {
            sample_rate: config.sample_rate.0,
            channels: config.channels,
            encoding: audio_encoding(sample_format)?,
        })
    }

    pub fn start(
        audio_tx: UnboundedSender<Vec<u8>>,
        selected_device: Option<&str>,
    ) -> Result<StartedRecording, String> {
        let stats = Arc::new(Mutex::new(AudioCaptureStats::default()));
        let level = Arc::new(AtomicU32::new(0));
        let (ready_tx, ready_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();

        let worker_stats = Arc::clone(&stats);
        let worker_level = Arc::clone(&level);
        let worker_device = selected_device.map(str::to_string);
        let worker = std::thread::Builder::new()
            .name("audio-capture".to_string())
            .spawn(move || {
                run_capture_loop(
                    worker_stats,
                    worker_level,
                    audio_tx,
                    ready_tx,
                    command_rx,
                    worker_device,
                )
            })
            .map_err(|error| format!("Could not spawn microphone capture thread: {error}"))?;

        let fell_back_to_default = ready_rx
            .recv()
            .map_err(|_| "Microphone capture thread exited unexpectedly.".to_string())??;

        Ok(StartedRecording {
            recorder: Self {
                stats,
                level,
                command_tx,
                worker: Some(worker),
                started_at: Instant::now(),
            },
            fell_back_to_default,
        })
    }

    /// Highest chunk RMS since the previous UI sample; reading clears it so
    /// missing callbacks settle to silence rather than holding a stale level.
    pub fn take_level(&self) -> f32 {
        f32::from_bits(self.level.swap(0, Ordering::Relaxed))
    }

    pub fn stop(mut self) -> AudioCaptureStats {
        self.shutdown();
        let mut stats = self.stats();
        stats.elapsed_ms = self.started_at.elapsed().as_millis();
        stats
    }

    fn shutdown(&mut self) {
        let _ = self.command_tx.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }

    fn stats(&self) -> AudioCaptureStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }
}

impl Drop for AudioRecorder {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_capture_loop(
    stats: Arc<Mutex<AudioCaptureStats>>,
    level: Arc<AtomicU32>,
    audio_tx: UnboundedSender<Vec<u8>>,
    ready_tx: mpsc::Sender<Result<bool, String>>,
    command_rx: mpsc::Receiver<()>,
    selected_device: Option<String>,
) {
    // cpal streams are !Send on some platforms (macOS CoreAudio), so the
    // device and stream must be created and dropped on this dedicated thread.
    let stream = match open_input_stream(stats, level, audio_tx, selected_device.as_deref()) {
        Ok((stream, fell_back_to_default)) => {
            if ready_tx.send(Ok(fell_back_to_default)).is_err() {
                return;
            }
            stream
        }
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };

    // Blocks until shutdown is requested; an error means every sender was
    // dropped without a stop signal, which is also a reason to clean up.
    let _ = command_rx.recv();

    drop(stream);
}

fn open_input_stream(
    stats: Arc<Mutex<AudioCaptureStats>>,
    level: Arc<AtomicU32>,
    audio_tx: UnboundedSender<Vec<u8>>,
    selected_device: Option<&str>,
) -> Result<(Stream, bool), String> {
    let (device, fell_back_to_default) = resolve_input_device(selected_device)?;
    let (config, sample_format) = input_config(&device)?;
    let stream = match sample_format {
        SampleFormat::F32 => build_input_stream::<f32>(&device, &config, stats, level, audio_tx)?,
        SampleFormat::I16 => build_input_stream::<i16>(&device, &config, stats, level, audio_tx)?,
        SampleFormat::U16 => build_input_stream::<u16>(&device, &config, stats, level, audio_tx)?,
        sample_format => {
            return Err(format!(
                "Microphone sample format {sample_format:?} is not supported yet."
            ));
        }
    };

    stream
        .play()
        .map_err(|error| format!("Could not start microphone capture: {error}"))?;

    Ok((stream, fell_back_to_default))
}

fn default_input_device(host: &cpal::Host) -> Result<cpal::Device, String> {
    host.default_input_device()
        .ok_or_else(|| "No microphone input device was found.".to_string())
}

/// Picks the configured device by name, falling back to the OS default
/// when nothing is configured or the configured device is missing.
fn resolve_input_device(selected: Option<&str>) -> Result<(cpal::Device, bool), String> {
    let host = cpal::default_host();

    if let Some(name) = selected {
        let configured = host.input_devices().ok().and_then(|mut devices| {
            devices.find(|device| device.name().map(|n| n == name).unwrap_or(false))
        });

        if let Some(device) = configured {
            return Ok((device, false));
        }
    }

    default_input_device(&host).map(|device| (device, selected.is_some()))
}

fn input_config(device: &cpal::Device) -> Result<(StreamConfig, SampleFormat), String> {
    let supported_config = device
        .default_input_config()
        .map_err(|error| format!("Could not read microphone input config: {error}"))?;
    let sample_format = supported_config.sample_format();
    Ok((supported_config.into(), sample_format))
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    stats: Arc<Mutex<AudioCaptureStats>>,
    level: Arc<AtomicU32>,
    audio_tx: UnboundedSender<Vec<u8>>,
) -> Result<Stream, String>
where
    T: PcmBytes + cpal::SizedSample + Send + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _| record_audio_chunk::<T>(data, &stats, &level, &audio_tx),
            move |error| eprintln!("Microphone stream error: {error}"),
            None,
        )
        .map_err(|error| format!("Could not open microphone input stream: {error}"))
}

fn record_audio_chunk<T>(
    data: &[T],
    stats: &Arc<Mutex<AudioCaptureStats>>,
    level: &AtomicU32,
    audio_tx: &UnboundedSender<Vec<u8>>,
) where
    T: PcmBytes,
{
    if data.is_empty() {
        return;
    }

    // Positive finite float bits are ordered like their numeric values.
    // The audio callback only computes a scalar and updates an atomic; IPC
    // happens on a separate task, never on this real-time capture thread.
    level.fetch_max(samples_rms(data).to_bits(), Ordering::Relaxed);

    let bytes = samples_to_le_bytes(data);

    if let Ok(mut stats) = stats.lock() {
        stats.chunk_count += 1;
        stats.sample_count += data.len() as u64;
        stats.byte_count += mem::size_of_val(data) as u64;
    }

    let _ = audio_tx.send(bytes);
}

fn audio_encoding(sample_format: SampleFormat) -> Result<AudioEncoding, String> {
    match sample_format {
        SampleFormat::F32 => Ok(AudioEncoding::F32),
        SampleFormat::I16 => Ok(AudioEncoding::I16),
        SampleFormat::U16 => Ok(AudioEncoding::U16),
        format => Err(format!(
            "Microphone sample format {format:?} is not supported yet."
        )),
    }
}

trait PcmBytes {
    fn append_le_bytes(&self, output: &mut Vec<u8>);
    fn normalized(&self) -> f32;
}

impl PcmBytes for f32 {
    fn normalized(&self) -> f32 {
        if self.is_finite() {
            self.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    fn append_le_bytes(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.to_le_bytes());
    }
}

impl PcmBytes for i16 {
    fn normalized(&self) -> f32 {
        f32::from(*self) / 32768.0
    }

    fn append_le_bytes(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.to_le_bytes());
    }
}

impl PcmBytes for u16 {
    fn normalized(&self) -> f32 {
        (f32::from(*self) - 32768.0) / 32768.0
    }

    fn append_le_bytes(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.to_le_bytes());
    }
}

fn samples_rms<T: PcmBytes>(samples: &[T]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples
        .iter()
        .map(|sample| f64::from(sample.normalized()).powi(2))
        .sum();
    (sum / samples.len() as f64).sqrt() as f32
}

fn samples_to_le_bytes<T: PcmBytes>(samples: &[T]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(mem::size_of_val(samples));

    for sample in samples {
        sample.append_le_bytes(&mut bytes);
    }

    bytes
}

#[cfg(test)]
mod level_tests {
    use super::*;

    #[test]
    fn measures_silence_and_volume_in_all_supported_formats() {
        assert_eq!(samples_rms::<f32>(&[]), 0.0);
        assert_eq!(samples_rms(&[0.0_f32; 32]), 0.0);
        assert_eq!(samples_rms(&[0_i16; 32]), 0.0);
        assert_eq!(samples_rms(&[32768_u16; 32]), 0.0);
        assert_eq!(samples_rms(&[-0.5_f32, 0.5]), 0.5);
        assert_eq!(samples_rms(&[-16384_i16, 16384]), 0.5);
        assert_eq!(samples_rms(&[16384_u16, 49152]), 0.5);
        assert!(samples_rms(&[-0.8_f32, 0.8]) > samples_rms(&[-0.1_f32, 0.1]));
        assert_eq!(samples_rms(&[i16::MIN]), 1.0);
        assert_eq!(samples_rms(&[0_u16]), 1.0);
        assert_eq!(samples_rms(&[f32::NAN, f32::INFINITY]), 0.0);
        assert_eq!(samples_rms(&[-2.0_f32, 2.0]), 1.0);
    }

    #[test]
    fn capture_updates_levels_without_changing_audio_and_reading_clears_the_peak() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let stats = Arc::new(Mutex::new(AudioCaptureStats::default()));
        let level = Arc::new(AtomicU32::new(0));
        record_audio_chunk(&[0.75_f32, -0.75], &stats, &level, &tx);
        record_audio_chunk(&[0.1_f32, -0.1], &stats, &level, &tx);
        assert_eq!(
            rx.try_recv().unwrap(),
            samples_to_le_bytes(&[0.75_f32, -0.75])
        );
        assert_eq!(
            rx.try_recv().unwrap(),
            samples_to_le_bytes(&[0.1_f32, -0.1])
        );
        let (command_tx, _) = mpsc::channel();
        let recorder = AudioRecorder {
            stats,
            level,
            command_tx,
            worker: None,
            started_at: Instant::now(),
        };
        assert_eq!(recorder.take_level(), 0.75);
        assert_eq!(recorder.take_level(), 0.0);
        record_audio_chunk(&[0.0_f32; 16], &recorder.stats, &recorder.level, &tx);
        assert_eq!(recorder.take_level(), 0.0);
        assert_eq!(recorder.stats().chunk_count, 3);
    }
}
