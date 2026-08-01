//! Decode / resample audio for `GigaAM` (16 kHz mono).

pub mod decode;
pub mod resample;

use std::path::Path;

use thiserror::Error;

pub const SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("failed to read audio: {0}")]
    Read(String),
    #[error("ffmpeg failed: {0}")]
    Ffmpeg(String),
    #[error("resample failed: {0}")]
    Resample(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Load any supported container → PCM f32 mono 16 kHz in `[-1, 1]`.
pub fn load_pcm16k_mono(path: &Path) -> Result<Vec<f32>, AudioError> {
    if !path.is_file() {
        return Err(AudioError::Read(format!("not found: {}", path.display())));
    }

    // Fast path: native WAV via hound.
    if is_wav(path) {
        match decode::decode_wav(path) {
            Ok((samples, sr)) => return resample::to_mono_16k(&samples, 1, sr),
            Err(AudioError::Read(_)) => {
                // Fall through to ffmpeg for odd WAVE variants.
            }
            Err(e) => return Err(e),
        }
    }

    decode::decode_ffmpeg(path)
}

fn is_wav(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
}
