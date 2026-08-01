//! HTK log-mel frontend for GigaAM.
//!
//! Matches official configs (`v3_*_ctc.yaml` / FeatureExtractor + SpecScaler):
//! 16 kHz, 64 mels, n_fft=win=320, hop=160, HTK, power=2, center=false, ln(clamp).
//!
//! Note: older notes sometimes cite n_fft=512; shipped GigaAM YAML uses 320.

use rustfft::{num_complex::Complex, FftPlanner};

/// GigaAM mel parameters (CTC / RNNT preprocessor).
#[derive(Debug, Clone, Copy)]
pub struct MelConfig {
    pub sample_rate: u32,
    pub n_mels: usize,
    pub n_fft: usize,
    pub win_length: usize,
    pub hop_length: usize,
    pub center: bool,
    pub f_min: f64,
    pub f_max: Option<f64>,
}

impl MelConfig {
    /// Defaults matching GigaAM FeatureExtractor YAML for ASR.
    pub const fn gigaam() -> Self {
        Self {
            sample_rate: 16_000,
            n_mels: 64,
            n_fft: 320,
            win_length: 320,
            hop_length: 160,
            center: false,
            f_min: 0.0,
            f_max: None,
        }
    }

    pub fn n_frames(&self, n_samples: usize) -> usize {
        if self.center {
            1 + n_samples / self.hop_length
        } else if n_samples < self.win_length {
            0
        } else {
            1 + (n_samples - self.win_length) / self.hop_length
        }
    }
}

/// Log-mel spectrogram: shape `[n_mels * n_frames]` in column-major time
/// (frame-major: frame0 all mels, frame1 …) — also expose as `(n_mels, time)`.
#[derive(Debug, Clone)]
pub struct MelSpectrogram {
    pub n_mels: usize,
    pub n_frames: usize,
    /// Row-major `[frame][mel]` flattened: index = frame * n_mels + mel.
    pub data: Vec<f32>,
}

impl MelSpectrogram {
    pub fn mel_at(&self, frame: usize, mel: usize) -> f32 {
        self.data[frame * self.n_mels + mel]
    }

    /// Channel layout like torchaudio: `[n_mels, time]` row-major.
    pub fn as_n_mels_by_time(&self) -> Vec<f32> {
        let mut out = vec![0.0; self.n_mels * self.n_frames];
        for t in 0..self.n_frames {
            for m in 0..self.n_mels {
                out[m * self.n_frames + t] = self.mel_at(t, m);
            }
        }
        out
    }
}

pub fn extract_log_mel(samples: &[f32], cfg: MelConfig) -> MelSpectrogram {
    let n_frames = cfg.n_frames(samples.len());
    let fbanks = htk_mel_filterbank(cfg);
    let window = hann_periodic(cfg.win_length);
    let n_freqs = cfg.n_fft / 2 + 1;

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(cfg.n_fft);

    let mut data = Vec::with_capacity(n_frames * cfg.n_mels);
    let mut buf = vec![Complex::new(0.0, 0.0); cfg.n_fft];

    for frame in 0..n_frames {
        let start = frame * cfg.hop_length;

        buf.fill(Complex::new(0.0, 0.0));
        for (i, w) in window.iter().enumerate() {
            let idx = start + i;
            let s = if idx < samples.len() {
                samples[idx]
            } else {
                0.0
            };
            buf[i] = Complex::new(s * w, 0.0);
        }
        fft.process(&mut buf);

        // Power spectrogram (onesided).
        let mut power = vec![0.0f32; n_freqs];
        for (k, bin) in power.iter_mut().enumerate() {
            let c = buf[k];
            *bin = c.re * c.re + c.im * c.im;
        }

        for m in 0..cfg.n_mels {
            let mut energy = 0.0f32;
            for (k, &p) in power.iter().enumerate() {
                energy = p.mul_add(fbanks[m * n_freqs + k], energy);
            }
            // SpecScaler: ln(clamp(x, 1e-9, 1e9))
            let clamped = energy.clamp(1e-9, 1e9);
            data.push(clamped.ln());
        }
    }

    MelSpectrogram {
        n_mels: cfg.n_mels,
        n_frames,
        data,
    }
}

fn hann_periodic(len: usize) -> Vec<f32> {
    // torch.hann_window(periodic=True)
    if len == 0 {
        return Vec::new();
    }
    let n = len as f64;
    (0..len)
        .map(|i| {
            let x = std::f64::consts::PI * 2.0 * i as f64 / n;
            0.5f64.mul_add(-x.cos(), 0.5) as f32
        })
        .collect()
}

pub fn hz_to_mel_htk(hz: f64) -> f64 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

pub fn mel_to_hz_htk(mel: f64) -> f64 {
    700.0 * (10f64.powf(mel / 2595.0) - 1.0)
}

/// Triangular HTK filterbank, no Slaney norm (`mel_norm: null`).
fn htk_mel_filterbank(cfg: MelConfig) -> Vec<f32> {
    let n_freqs = cfg.n_fft / 2 + 1;
    let f_max = cfg
        .f_max
        .unwrap_or_else(|| f64::from(cfg.sample_rate) / 2.0);
    let mel_min = hz_to_mel_htk(cfg.f_min);
    let mel_max = hz_to_mel_htk(f_max);
    let mel_points: Vec<f64> = (0..cfg.n_mels + 2)
        .map(|i| mel_min + (mel_max - mel_min) * i as f64 / (cfg.n_mels as f64 + 1.0))
        .collect();
    let hz_points: Vec<f64> = mel_points.iter().copied().map(mel_to_hz_htk).collect();
    let bins: Vec<f64> = hz_points
        .iter()
        .map(|hz| (cfg.n_fft as f64 + 1.0) * hz / f64::from(cfg.sample_rate))
        .collect();

    let mut fbanks = vec![0.0f32; cfg.n_mels * n_freqs];
    for m in 1..=cfg.n_mels {
        let left = bins[m - 1];
        let center = bins[m];
        let right = bins[m + 1];
        for k in 0..n_freqs {
            let k_f = k as f64;
            let mut weight = 0.0;
            if k_f >= left && k_f <= center && (center - left) > 0.0 {
                weight = (k_f - left) / (center - left);
            } else if k_f >= center && k_f <= right && (right - center) > 0.0 {
                weight = (right - k_f) / (right - center);
            }
            fbanks[(m - 1) * n_freqs + k] = weight as f32;
        }
    }
    fbanks
}
