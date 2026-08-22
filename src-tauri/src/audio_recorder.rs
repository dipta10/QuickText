use std::{
    mem,
    sync::{Arc, Mutex},
    time::Instant,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};
use serde::Serialize;

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

pub struct AudioRecorder {
    stream: Stream,
    stats: Arc<Mutex<AudioCaptureStats>>,
    started_at: Instant,
    format: AudioFormat,
}

impl AudioRecorder {
    pub fn start() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No microphone input device was found.".to_string())?;
        let supported_config = device
            .default_input_config()
            .map_err(|error| format!("Could not read microphone input config: {error}"))?;
        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();
        let stats = Arc::new(Mutex::new(AudioCaptureStats::default()));
        let stream = match sample_format {
            SampleFormat::F32 => build_input_stream::<f32>(&device, &config, Arc::clone(&stats))?,
            SampleFormat::I16 => build_input_stream::<i16>(&device, &config, Arc::clone(&stats))?,
            SampleFormat::U16 => build_input_stream::<u16>(&device, &config, Arc::clone(&stats))?,
            format => {
                return Err(format!(
                    "Microphone sample format {format:?} is not supported yet."
                ));
            }
        };

        stream
            .play()
            .map_err(|error| format!("Could not start microphone capture: {error}"))?;

        Ok(Self {
            stream,
            stats,
            started_at: Instant::now(),
            format: AudioFormat {
                sample_rate: config.sample_rate.0,
                channels: config.channels,
                encoding: audio_encoding(sample_format)?,
            },
        })
    }

    pub fn format(&self) -> AudioFormat {
        self.format.clone()
    }

    pub fn stop(self) -> AudioCaptureStats {
        let mut stats = self.stats();
        stats.elapsed_ms = self.started_at.elapsed().as_millis();
        drop(self.stream);
        stats
    }

    fn stats(&self) -> AudioCaptureStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    stats: Arc<Mutex<AudioCaptureStats>>,
) -> Result<Stream, String>
where
    T: cpal::SizedSample + Send + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _| record_audio_chunk::<T>(data, &stats),
            move |error| eprintln!("Microphone stream error: {error}"),
            None,
        )
        .map_err(|error| format!("Could not open microphone input stream: {error}"))
}

fn record_audio_chunk<T>(data: &[T], stats: &Arc<Mutex<AudioCaptureStats>>) {
    if data.is_empty() {
        return;
    }

    if let Ok(mut stats) = stats.lock() {
        stats.chunk_count += 1;
        stats.sample_count += data.len() as u64;
        stats.byte_count += (data.len() * mem::size_of::<T>()) as u64;
    }
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
