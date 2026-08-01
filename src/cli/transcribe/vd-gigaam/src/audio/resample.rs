//! Resample → 16 kHz mono.

use super::{AudioError, SAMPLE_RATE};

/// `samples` are already mono (or downmixed here). Resample to 16 kHz when needed.
pub fn to_mono_16k(
    samples: &[f32],
    channels: u16,
    sample_rate: u32,
) -> Result<Vec<f32>, AudioError> {
    let mono = if channels <= 1 {
        samples.to_vec()
    } else {
        downmix(samples, channels as usize)
    };

    if sample_rate == SAMPLE_RATE {
        return Ok(mono);
    }
    if mono.is_empty() {
        return Ok(mono);
    }
    resample_linear(&mono, sample_rate, SAMPLE_RATE)
}

fn downmix(samples: &[f32], channels: usize) -> Vec<f32> {
    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Deterministic linear resample (length = round(n * to/from)).
pub fn resample_linear(samples: &[f32], from: u32, to: u32) -> Result<Vec<f32>, AudioError> {
    if from == to {
        return Ok(samples.to_vec());
    }
    if from == 0 {
        return Err(AudioError::Resample("sample rate is 0".into()));
    }
    let ratio = f64::from(to) / f64::from(from);
    let out_len = ((samples.len() as f64) * ratio).round().max(0.0) as usize;
    if out_len == 0 || samples.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(out_len);
    let last = samples.len() - 1;
    for i in 0..out_len {
        let src = (i as f64) / ratio;
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(last);
        let t = (src - i0 as f64) as f32;
        let s0 = samples[i0.min(last)];
        let s1 = samples[i1];
        out.push(s0.mul_add(1.0 - t, s1 * t));
    }
    Ok(out)
}
