use std::path::{Path, PathBuf};

use crate::{
    audio_recorder::{AudioEncoding, AudioFormat},
    soniox_provider::SonioxSession,
};

const FIXTURES_DIR_NAME: &str = "fixtures";
const AUDIO_CHUNK_BYTES: usize = 4096;

struct TranscriptionFixture {
    wav_path: PathBuf,
    expected_transcript: String,
}

fn normalize_transcript(text: &str) -> String {
    let without_markers = text.replace("<end>", " ").replace("<fin>", " ");
    let lowered = without_markers.to_lowercase();
    let kept: String = lowered
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == '\'' || *character == ' ')
        .collect();

    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn list_fixtures(fixtures_dir: &Path) -> Result<Vec<TranscriptionFixture>, String> {
    let mut fixtures = Vec::new();

    let entries = std::fs::read_dir(fixtures_dir)
        .map_err(|error| format!("Could not read fixtures directory: {error}"))?;

    for entry in entries {
        let path = entry
            .map_err(|error| format!("Could not read fixture entry: {error}"))?
            .path();

        if path.extension().is_some_and(|extension| extension == "wav") {
            let expected_path = path.with_extension("expected.txt");
            let expected_transcript = std::fs::read_to_string(&expected_path).map_err(|error| {
                format!(
                    "Missing expected transcript for {}: {error}",
                    path.display()
                )
            })?;

            fixtures.push(TranscriptionFixture {
                wav_path: path,
                expected_transcript,
            });
        }
    }

    if fixtures.is_empty() {
        return Err(format!(
            "No .wav fixtures found in {}",
            fixtures_dir.display()
        ));
    }

    fixtures.sort_by_key(|fixture| fixture.wav_path.clone());
    Ok(fixtures)
}

fn load_wav_pcm(wav_path: &Path) -> Result<(Vec<u8>, AudioFormat), String> {
    let mut reader = hound::WavReader::open(wav_path)
        .map_err(|error| format!("Could not open {}: {error}", wav_path.display()))?;
    let spec = reader.spec();

    let encoding;
    let mut pcm_bytes = Vec::new();

    match spec.sample_format {
        hound::SampleFormat::Float => {
            encoding = AudioEncoding::F32;

            for sample in reader.samples::<f32>() {
                let sample = sample
                    .map_err(|error| format!("Could not read {}: {error}", wav_path.display()))?;
                pcm_bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }
        hound::SampleFormat::Int => {
            if spec.bits_per_sample != 16 {
                return Err(format!(
                    "{} uses {}-bit samples; only 16-bit PCM and 32-bit float WAV files are supported.",
                    wav_path.display(),
                    spec.bits_per_sample
                ));
            }

            encoding = AudioEncoding::I16;

            for sample in reader.samples::<i16>() {
                let sample = sample
                    .map_err(|error| format!("Could not read {}: {error}", wav_path.display()))?;
                pcm_bytes.extend_from_slice(&sample.to_le_bytes());
            }
        }
    }

    Ok((
        pcm_bytes,
        AudioFormat {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            encoding,
        },
    ))
}

fn test_api_key() -> Result<String, String> {
    std::env::var("QUICKTEXT_TEST_API_KEY")
        .or_else(|_| std::env::var("SONIOX_API_KEY"))
        .map_err(|_| {
            "Set QUICKTEXT_TEST_API_KEY (or SONIOX_API_KEY) to run transcription fixture tests."
                .to_string()
        })
}

fn chunk_duration(audio_format: &AudioFormat) -> std::time::Duration {
    let bytes_per_sample = match audio_format.encoding {
        AudioEncoding::F32 => 4,
        AudioEncoding::I16 | AudioEncoding::U16 => 2,
    };

    let samples_per_channel =
        AUDIO_CHUNK_BYTES / (bytes_per_sample * audio_format.channels as usize);
    let micros = (samples_per_channel as u64 * 1_000_000) / audio_format.sample_rate as u64;
    std::time::Duration::from_micros(micros)
}

#[tokio::test]
#[ignore = "requires network access, an API key, and recorded fixtures; run with `cargo test -- --ignored`"]
async fn transcribes_recorded_fixtures() {
    let api_key = match test_api_key() {
        Ok(key) => key,
        Err(missing_key_message) => panic!("{missing_key_message}"),
    };
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURES_DIR_NAME);

    for fixture in list_fixtures(&fixtures_dir).expect("valid fixture set") {
        let (pcm_bytes, audio_format) =
            load_wav_pcm(&fixture.wav_path).expect("readable WAV fixture");

        let chunk_duration = chunk_duration(&audio_format);
        let session = SonioxSession::start(api_key.clone(), audio_format)
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "Could not start session for {}: {error}",
                    fixture.wav_path.display()
                )
            });

        let audio_sender = session.audio_sender();
        for chunk in pcm_bytes.chunks(AUDIO_CHUNK_BYTES) {
            audio_sender
                .send(chunk.to_vec())
                .expect("session audio channel stays open while streaming");
            tokio::time::sleep(chunk_duration).await;
        }
        drop(audio_sender);

        let transcript = session.stop().await.unwrap_or_else(|error| {
            panic!(
                "Transcription failed for {}: {error}",
                fixture.wav_path.display()
            )
        });

        assert!(
            !transcript.text.contains("<end>") && !transcript.text.contains("<fin>"),
            "stream markers leaked into final transcript: {:?}",
            transcript.text
        );

        assert_eq!(
            normalize_transcript(&transcript.text),
            normalize_transcript(&fixture.expected_transcript),
            "transcript mismatch for {} (raw provider output: {:?})",
            fixture.wav_path.display(),
            transcript.text
        );
    }
}

#[test]
fn normalization_is_case_and_punctuation_insensitive() {
    assert_eq!(
        normalize_transcript("Hi there, it's a beautiful day."),
        "hi there it's a beautiful day"
    );
}

#[test]
fn normalization_strips_stream_markers() {
    assert_eq!(
        normalize_transcript("<end><fin>hello world<end>"),
        "hello world"
    );
}

#[test]
fn normalization_collapses_whitespace() {
    assert_eq!(
        normalize_transcript("  spaced   out \t text "),
        "spaced out text"
    );
}
