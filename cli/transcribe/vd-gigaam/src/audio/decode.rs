//! Decode WAV via `hound`; other containers via `ffmpeg` (same idea as GigaAM).

use std::path::Path;
use std::process::Command;

use hound::{SampleFormat, WavReader};

use super::{AudioError, SAMPLE_RATE};

/// Decode WAV → mono f32 samples + sample rate.
pub fn decode_wav(path: &Path) -> Result<(Vec<f32>, u32), AudioError> {
    let mut reader = WavReader::open(path).map_err(|e| AudioError::Read(e.to_string()))?;
    let spec = reader.spec();
    let sr = spec.sample_rate;
    let channels = spec.channels as usize;
    if channels == 0 {
        return Err(AudioError::Read("WAV has zero channels".into()));
    }

    let interleaved: Vec<f32> = match spec.sample_format {
        SampleFormat::Int => {
            let max = match spec.bits_per_sample {
                8 => i8::MAX as f32,
                16 => i16::MAX as f32,
                24 => ((1i32 << 23) - 1) as f32,
                32 => i32::MAX as f32,
                other => {
                    return Err(AudioError::Read(format!(
                        "unsupported WAV bit depth: {other}"
                    )));
                }
            };
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AudioError::Read(e.to_string()))?
        }
        SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AudioError::Read(e.to_string()))?,
    };

    Ok((to_mono(&interleaved, channels), sr))
}

fn to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Decode via ffmpeg to s16le mono @ 16 kHz (matches GigaAM `load_audio`).
pub fn decode_ffmpeg(path: &Path) -> Result<Vec<f32>, AudioError> {
    let path_str = path
        .to_str()
        .ok_or_else(|| AudioError::Read("non-utf8 path".into()))?;
    let output = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-threads",
            "0",
            "-i",
            path_str,
            "-f",
            "s16le",
            "-ac",
            "1",
            "-acodec",
            "pcm_s16le",
            "-ar",
            &SAMPLE_RATE.to_string(),
            "-",
        ])
        .output()
        .map_err(|e| AudioError::Ffmpeg(e.to_string()))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AudioError::Ffmpeg(err.trim().to_string()));
    }

    let bytes = output.stdout;
    if bytes.len() % 2 != 0 {
        return Err(AudioError::Ffmpeg("odd pcm byte length".into()));
    }
    Ok(bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
        .collect())
}
