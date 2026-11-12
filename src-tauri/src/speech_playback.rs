//! Native streaming playback. The audio stream stays on its owning thread.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub const SAMPLE_RATE: u32 = 24_000;
const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 125;

#[derive(Default)]
struct Buffer {
    samples: VecDeque<i16>,
    phase: f64,
    ended: bool,
    received: usize,
}
impl Buffer {
    fn sample(&mut self, step: f64) -> f32 {
        // Linear interpolation; retain the last sample until its successor arrives.
        let Some(&first) = self.samples.front() else {
            return 0.0;
        };
        let second = self.samples.get(1).copied().unwrap_or(first);
        let value = first as f64 + (second as f64 - first as f64) * self.phase;
        self.phase += step;
        while self.phase >= 1.0 {
            if self.samples.pop_front().is_none() {
                self.phase = 0.0;
                break;
            }
            self.phase -= 1.0;
        }
        value as f32 / 32768.0
    }
}

pub struct AudioPlayback {
    buffer: Arc<Mutex<Buffer>>,
    stopped: Arc<AtomicBool>,
    finished: tokio::sync::oneshot::Receiver<Result<(), String>>,
}
impl Drop for AudioPlayback {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
    }
}
impl AudioPlayback {
    pub async fn start() -> Result<Self, String> {
        let buffer = Arc::new(Mutex::new(Buffer::default()));
        let stopped = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (done_tx, finished) = tokio::sync::oneshot::channel();
        let queue = buffer.clone();
        let stop = stopped.clone();
        let task_failure = failed.clone();
        std::thread::spawn(move || {
            let stream = open_stream(queue.clone(), stop.clone(), task_failure.clone());
            match stream {
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
                Ok(stream) => {
                    if ready_tx.send(Ok(())).is_err() {
                        return;
                    }
                    let result = loop {
                        if stop.load(Ordering::Acquire) {
                            break Ok(());
                        }
                        if task_failure.load(Ordering::Acquire) {
                            break Err(
                                "Audio output was disconnected. Check your speakers and try again."
                                    .into(),
                            );
                        }
                        let drained = queue
                            .lock()
                            .map(|q| q.ended && q.samples.is_empty())
                            .unwrap_or(true);
                        if drained {
                            // Let the final device buffer reach the speakers before dropping the stream.
                            std::thread::sleep(Duration::from_millis(100));
                            break Ok(());
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    };
                    drop(stream);
                    let _ = done_tx.send(result);
                }
            }
        });
        // Construct the guard before awaiting: cancellation during device setup stops its thread.
        let playback = Self {
            buffer,
            stopped,
            finished,
        };
        tokio::time::timeout(Duration::from_secs(5), ready_rx)
            .await
            .map_err(|_| "Starting audio output timed out.".to_string())?
            .map_err(|_| "Could not start audio output.".to_string())??;
        Ok(playback)
    }

    pub fn push(&self, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() % 2 != 0 {
            return Err("The speech service returned invalid audio. Try again.".into());
        }
        let mut queue = self
            .buffer
            .lock()
            .map_err(|_| "Audio playback is unavailable.".to_string())?;
        queue.received += bytes.len() / 2;
        if queue.received > MAX_SAMPLES {
            return Err("This reading is too long. Select a shorter passage.".into());
        }
        queue.samples.extend(
            bytes
                .chunks_exact(2)
                .map(|s| i16::from_le_bytes([s[0], s[1]])),
        );
        Ok(())
    }

    pub async fn finish(&mut self) -> Result<(), String> {
        {
            let mut queue = self
                .buffer
                .lock()
                .map_err(|_| "Audio playback is unavailable.".to_string())?;
            if queue.received == 0 {
                return Err("No speech was returned. Try again with a short passage.".into());
            }
            queue.ended = true;
        }
        (&mut self.finished)
            .await
            .map_err(|_| "Audio playback stopped unexpectedly.".to_string())?
    }
}

fn open_stream(
    buffer: Arc<Mutex<Buffer>>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or_else(|| {
            "No audio output is available. Connect speakers or headphones.".to_string()
        })?;
    let supported = device
        .default_output_config()
        .map_err(|_| "Could not open the default audio output.".to_string())?;
    let config = supported.config();
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&device, &config, buffer, stop, failed),
        cpal::SampleFormat::I16 => build::<i16>(&device, &config, buffer, stop, failed),
        cpal::SampleFormat::U16 => build::<u16>(&device, &config, buffer, stop, failed),
        _ => {
            return Err(
                "This audio output format is not supported. Choose a different system output."
                    .into(),
            )
        }
    }
    .map_err(|_| "Could not start audio output. Check your system sound settings.".to_string())?;
    stream
        .play()
        .map_err(|_| "Could not play audio through the default output.".to_string())?;
    Ok(stream)
}
fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Buffer>>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    let channels = config.channels as usize;
    let step = SAMPLE_RATE as f64 / config.sample_rate.0 as f64;
    device.build_output_stream(
        config,
        move |output: &mut [T], _| {
            let mut queue = buffer.try_lock().ok();
            for frame in output.chunks_mut(channels) {
                let value = if stop.load(Ordering::Acquire) {
                    0.0
                } else {
                    queue.as_mut().map(|q| q.sample(step)).unwrap_or(0.0)
                };
                frame.fill(T::from_sample(value));
            }
        },
        move |_| {
            failed.store(true, Ordering::Release);
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resamples_mono_without_changing_duration_and_drains() {
        let mut buffer = Buffer {
            samples: VecDeque::from([0, 16384, 0, -16384]),
            ..Default::default()
        };
        let output: Vec<_> = (0..8).map(|_| buffer.sample(0.5)).collect();
        assert_eq!(output, [0.0, 0.25, 0.5, 0.25, 0.0, -0.25, -0.5, -0.5]);
        assert!(buffer.samples.is_empty());
        assert_eq!(buffer.sample(0.5), 0.0);
    }
}
