//! Native streaming playback. DSP and the audio stream stay on their owning thread.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use libsonic::SonicStream;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub const SAMPLE_RATE: u32 = 24_000;
const MAX_SAMPLES: usize = SAMPLE_RATE as usize * 125;
const PROCESS_CHUNK: usize = SAMPLE_RATE as usize / 50;
// Keep processed audio short so a speed change reaches the speakers promptly.
const OUTPUT_AHEAD: usize = SAMPLE_RATE as usize / 10;

pub fn validate_speed(speed: f32) -> Result<(), String> {
    if !speed.is_finite() || !(0.5..=2.0).contains(&speed) {
        return Err("Playback speed must be between 0.5 and 2.0×.".into());
    }
    Ok(())
}

#[derive(Default)]
struct Input {
    samples: VecDeque<f32>,
    ended: bool,
    received: usize,
}
#[derive(Default)]
struct Buffer {
    samples: VecDeque<f32>,
    phase: f64,
    ended: bool,
}
impl Buffer {
    fn sample(&mut self, step: f64) -> f32 {
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
        value as f32
    }
}

struct Tempo {
    stream: SonicStream,
    flushed: bool,
}
impl Tempo {
    fn new() -> Result<Self, String> {
        Ok(Self {
            stream: SonicStream::new(SAMPLE_RATE, 1)
                .map_err(|_| "Could not prepare playback speed control.".to_string())?,
            flushed: false,
        })
    }
    fn process(&mut self, input: &mut Input, output: &mut Buffer, speed: f32) {
        if self.flushed || output.samples.len() >= OUTPUT_AHEAD {
            return;
        }
        self.stream.set_speed(speed);
        let count = input.samples.len().min(PROCESS_CHUNK);
        if count > 0 {
            let samples: Vec<_> = input.samples.drain(..count).collect();
            self.stream.write_f32(&samples);
        }
        if input.ended && input.samples.is_empty() {
            self.stream.flush();
            self.flushed = true;
        }
        output
            .samples
            .extend(self.stream.read_f32(self.stream.available_frames()));
        output.ended = self.flushed;
    }
}

pub struct AudioPlayback {
    input: Arc<Mutex<Input>>,
    stopped: Arc<AtomicBool>,
    finished: tokio::sync::oneshot::Receiver<Result<(), String>>,
}
impl Drop for AudioPlayback {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
    }
}
impl AudioPlayback {
    pub async fn start(speed: Arc<AtomicU32>) -> Result<Self, String> {
        let input = Arc::new(Mutex::new(Input::default()));
        let stopped = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let (done_tx, finished) = tokio::sync::oneshot::channel();
        let source = input.clone();
        let stop = stopped.clone();
        std::thread::spawn(move || {
            let output = Arc::new(Mutex::new(Buffer::default()));
            let setup = Tempo::new().and_then(|tempo| {
                open_stream(output.clone(), stop.clone(), failed.clone())
                    .map(|stream| (tempo, stream))
            });
            match setup {
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
                Ok((mut tempo, stream)) => {
                    if ready_tx.send(Ok(())).is_err() {
                        return;
                    }
                    let result = loop {
                        if stop.load(Ordering::Acquire) {
                            break Ok(());
                        }
                        if failed.load(Ordering::Acquire) {
                            break Err(
                                "Audio output was disconnected. Check your speakers and try again."
                                    .into(),
                            );
                        }
                        let ahead = match output.lock() {
                            Ok(output) => output.samples.len(),
                            Err(_) => break Err("Audio playback is unavailable.".into()),
                        };
                        if ahead < OUTPUT_AHEAD && !tempo.flushed {
                            // Do the pitch analysis outside the output lock. The
                            // audio callback must never wait for DSP work.
                            let mut processed = Buffer::default();
                            match source.lock() {
                                Ok(mut input) => tempo.process(
                                    &mut input,
                                    &mut processed,
                                    f32::from_bits(speed.load(Ordering::Acquire)),
                                ),
                                Err(_) => break Err("Audio playback is unavailable.".into()),
                            }
                            match output.lock() {
                                Ok(mut output) => {
                                    output.samples.extend(processed.samples);
                                    output.ended = processed.ended;
                                }
                                Err(_) => break Err("Audio playback is unavailable.".into()),
                            }
                        }
                        let drained = output
                            .lock()
                            .map(|output| output.ended && output.samples.is_empty())
                            .unwrap_or(true);
                        if drained {
                            std::thread::sleep(Duration::from_millis(100));
                            break Ok(());
                        }
                        std::thread::sleep(Duration::from_millis(2));
                    };
                    drop(stream);
                    let _ = done_tx.send(result);
                }
            }
        });
        let playback = Self {
            input,
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
        let mut input = self
            .input
            .lock()
            .map_err(|_| "Audio playback is unavailable.".to_string())?;
        input.received += bytes.len() / 2;
        if input.received > MAX_SAMPLES {
            return Err("This reading is too long. Select a shorter passage.".into());
        }
        input.samples.extend(
            bytes
                .chunks_exact(2)
                .map(|s| i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0),
        );
        Ok(())
    }
    pub async fn finish(&mut self) -> Result<(), String> {
        {
            let mut input = self
                .input
                .lock()
                .map_err(|_| "Audio playback is unavailable.".to_string())?;
            if input.received == 0 {
                return Err("No speech was returned. Try again with a short passage.".into());
            }
            input.ended = true;
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
            samples: VecDeque::from([0.0, 0.5, 0.0, -0.5]),
            ..Default::default()
        };
        let output: Vec<_> = (0..8).map(|_| buffer.sample(0.5)).collect();
        assert_eq!(output, [0.0, 0.25, 0.5, 0.25, 0.0, -0.25, -0.5, -0.5]);
        assert!(buffer.samples.is_empty());
        assert_eq!(buffer.sample(0.5), 0.0);
    }
    fn render_tone(speed: impl Fn(usize) -> f32) -> Vec<f32> {
        let samples: VecDeque<_> = (0..SAMPLE_RATE * 2)
            .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / SAMPLE_RATE as f32).sin() * 0.5)
            .collect();
        let mut input = Input {
            samples,
            ended: true,
            ..Default::default()
        };
        let mut buffer = Buffer::default();
        let mut tempo = Tempo::new().unwrap();
        let mut result = Vec::new();
        for _ in 0..10000 {
            let remaining = input.samples.len();
            tempo.process(&mut input, &mut buffer, speed(remaining));
            result.extend(buffer.samples.drain(..));
            if buffer.ended {
                return result;
            }
        }
        panic!("Playback failed to drain");
    }
    #[test]
    fn speed_changes_duration_while_preserving_voice_pitch() {
        for speed in [0.5, 1.0, 1.5, 2.0] {
            let output = render_tone(|_| speed);
            let expected_length = SAMPLE_RATE as f32 * 2.0 / speed;
            assert!((output.len() as f32 - expected_length).abs() < expected_length * 0.05);
            let middle = &output[output.len() / 4..output.len() * 3 / 4];
            let crossings = middle
                .windows(2)
                .filter(|s| s[0] <= 0.0 && s[1] > 0.0)
                .count();
            let pitch = crossings as f32 * SAMPLE_RATE as f32 / middle.len() as f32;
            assert!((pitch - 440.0).abs() < 15.0, "pitch at {speed}×: {pitch}");
            assert!(output.iter().all(|sample| sample.is_finite()));
        }
    }
    #[test]
    fn changing_speed_mid_stream_keeps_all_remaining_audio() {
        let output = render_tone(|remaining| {
            if remaining > SAMPLE_RATE as usize {
                1.0
            } else {
                2.0
            }
        });
        let expected = SAMPLE_RATE as f32 * 1.5;
        assert!((output.len() as f32 - expected).abs() < expected * 0.05);
    }
    #[test]
    fn validates_rate_boundaries_and_nonfinite_values() {
        for speed in [0.5, 1.0, 2.0] {
            assert!(validate_speed(speed).is_ok());
        }
        for speed in [0.0, 0.49, 2.01, f32::NAN, f32::INFINITY] {
            assert!(validate_speed(speed).is_err());
        }
    }
}
