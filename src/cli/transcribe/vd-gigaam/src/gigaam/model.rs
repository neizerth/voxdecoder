//! Public inference API: `GigaModel::load` / `.transcribe`.

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use serde::Serialize;

use super::chunk;
use super::config::{GigaLoadOptions, TranscribeOptions};
use super::decoder::ctc;
use super::encoder::conformer::{ConformerConfig, ConformerEncoder, CtcHead};
use super::frontend::mel::{self, MelConfig};
use super::weights::{self, ModelCard, WeightsError};

#[derive(Debug, Clone, Serialize)]
pub struct Word {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    pub text: String,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Transcript {
    pub text: String,
    pub segments: Vec<Segment>,
    pub words: Option<Vec<Word>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error(transparent)]
    Weights(#[from] WeightsError),
    #[error("model load failed: {0}")]
    Load(String),
    #[error("transcription failed: {0}")]
    Transcribe(String),
}

pub struct GigaModel {
    card: ModelCard,
    encoder: ConformerEncoder,
    head: CtcHead,
    device: Device,
    mel: MelConfig,
}

impl GigaModel {
    pub fn load(options: GigaLoadOptions) -> Result<Self, ModelError> {
        let paths = weights::resolve_converted(&options.download_root, &options.model)
            .map_err(|e| match e {
                WeightsError::NotFound(p) => ModelError::Load(format!(
                    "converted SafeTensors not found at {} — run scripts/convert_ckpt.py on the .ckpt",
                    p.display()
                )),
                other => ModelError::Weights(other),
            })?;

        let card = ModelCard::load(&paths.card).map_err(ModelError::Load)?;
        if card.decoder != "ctc" {
            return Err(ModelError::Load(format!(
                "decoder '{}' not supported yet (only ctc)",
                card.decoder
            )));
        }

        let device = pick_device(&options)?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[paths.safetensors.as_path()], DType::F32, &device)
                .map_err(|e| ModelError::Load(e.to_string()))?
        };

        let enc_cfg = ConformerConfig::from_card(&card);
        let encoder = ConformerEncoder::load(vb.pp("encoder"), enc_cfg, &device)
            .map_err(|e| ModelError::Load(e.to_string()))?;
        let head = CtcHead::load(vb.pp("head"), card.head.feat_in, card.head.num_classes)
            .map_err(|e| ModelError::Load(e.to_string()))?;

        let mel = MelConfig {
            sample_rate: card.preprocessor.sample_rate,
            n_mels: card.preprocessor.features,
            n_fft: card.preprocessor.n_fft,
            win_length: card.preprocessor.win_length,
            hop_length: card.preprocessor.hop_length,
            center: card.preprocessor.center,
            f_min: 0.0,
            f_max: None,
        };

        Ok(Self {
            card,
            encoder,
            head,
            device,
            mel,
        })
    }

    /// Extract GigaAM log-mel features (public for golden / layer tests).
    pub fn extract_features(samples: &[f32]) -> mel::MelSpectrogram {
        mel::extract_log_mel(samples, MelConfig::gigaam())
    }

    pub fn transcribe(
        &self,
        samples: &[f32],
        opts: TranscribeOptions,
    ) -> Result<Transcript, ModelError> {
        self.transcribe_with_progress(samples, opts, |_, _| {})
    }

    /// Like [`Self::transcribe`], calling `on_chunk(1-based index, total)` after each window.
    pub fn transcribe_with_progress<F>(
        &self,
        samples: &[f32],
        opts: TranscribeOptions,
        mut on_chunk: F,
    ) -> Result<Transcript, ModelError>
    where
        F: FnMut(u32, u32),
    {
        let ranges = chunk::chunk_ranges(samples.len(), chunk::MAX_CHUNK_SAMPLES);
        if ranges.is_empty() {
            return Ok(Transcript {
                text: String::new(),
                segments: Vec::new(),
                words: None,
            });
        }
        let n = ranges.len() as u32;
        if ranges.len() == 1 {
            let out = self.transcribe_window(samples, 0.0, opts)?;
            on_chunk(1, n);
            return Ok(out);
        }

        let sr = f64::from(self.mel.sample_rate);
        let mut texts = Vec::new();
        let mut segments = Vec::new();
        let mut words = if opts.word_timestamps {
            Some(Vec::new())
        } else {
            None
        };

        for (i, (start, end)) in ranges.into_iter().enumerate() {
            let offset_sec = start as f64 / sr;
            let part = self.transcribe_window(&samples[start..end], offset_sec, opts.clone())?;
            if !part.text.is_empty() {
                texts.push(part.text);
            }
            segments.extend(part.segments);
            if let (Some(acc), Some(w)) = (words.as_mut(), part.words) {
                acc.extend(w);
            }
            // Flush Metal command buffers so pooled temps can be reused between chunks.
            let _ = self.device.synchronize();
            on_chunk((i as u32) + 1, n);
        }

        Ok(Transcript {
            text: texts.join(" "),
            segments,
            words,
        })
    }

    fn transcribe_window(
        &self,
        samples: &[f32],
        offset_sec: f64,
        opts: TranscribeOptions,
    ) -> Result<Transcript, ModelError> {
        let features = mel::extract_log_mel(samples, self.mel);
        if features.n_frames == 0 {
            return Ok(Transcript {
                text: String::new(),
                segments: Vec::new(),
                words: None,
            });
        }

        // Mel layout for encoder: [B, T, F]
        let mel_btf = features_to_btf(&features);
        let feat = Tensor::from_vec(
            mel_btf,
            (1, features.n_frames, features.n_mels),
            &self.device,
        )
        .map_err(|e| ModelError::Transcribe(e.to_string()))?;

        let (encoded, enc_len) = self
            .encoder
            .forward(&feat, features.n_frames)
            .map_err(|e| ModelError::Transcribe(e.to_string()))?;

        let log_probs = self
            .head
            .forward(&encoded)
            .map_err(|e| ModelError::Transcribe(e.to_string()))?;

        // [1, T, C] → argmax over C
        let labels = log_probs
            .argmax(candle_core::D::Minus1)
            .map_err(|e| ModelError::Transcribe(e.to_string()))?
            .squeeze(0)
            .map_err(|e| ModelError::Transcribe(e.to_string()))?
            .to_vec1::<u32>()
            .map_err(|e| ModelError::Transcribe(e.to_string()))?;

        let labels = &labels[..enc_len.min(labels.len())];
        let blank = self.card.blank_id();
        let (token_ids, token_frames) = ctc::greedy_collapse_with_frames(labels, blank);
        let text = self.card.decode_tokens(&token_ids);

        let words = if opts.word_timestamps {
            Some(
                frames_to_words(
                    &self.card,
                    &token_ids,
                    &token_frames,
                    samples.len(),
                    enc_len,
                )
                .into_iter()
                .map(|mut w| {
                    w.start += offset_sec;
                    w.end += offset_sec;
                    w
                })
                .collect(),
            )
        } else {
            None
        };

        let segments = if text.is_empty() {
            Vec::new()
        } else {
            let dur = samples.len() as f64 / f64::from(self.mel.sample_rate);
            vec![Segment {
                text: text.clone(),
                start: offset_sec,
                end: offset_sec + dur,
            }]
        };

        Ok(Transcript {
            text,
            segments,
            words,
        })
    }
}

fn features_to_btf(mel: &mel::MelSpectrogram) -> Vec<f32> {
    // data is frame-major [T * n_mels]; already [T, F] row-major.
    mel.data.clone()
}

fn frames_to_words(
    card: &ModelCard,
    token_ids: &[u32],
    token_frames: &[usize],
    n_samples: usize,
    enc_len: usize,
) -> Vec<Word> {
    if token_ids.is_empty() || enc_len == 0 {
        return Vec::new();
    }
    let frame_shift = n_samples as f64 / f64::from(16_000) / enc_len as f64;
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut start_f = 0usize;
    let mut last_f = 0usize;

    let flush = |cur: &mut String, start_f: usize, last_f: usize, words: &mut Vec<Word>| {
        let t = cur.replace('▁', " ").trim().to_string();
        if !t.is_empty() {
            words.push(Word {
                text: t,
                start: start_f as f64 * frame_shift,
                end: (last_f + 1) as f64 * frame_shift,
            });
        }
        cur.clear();
    };

    for (i, &id) in token_ids.iter().enumerate() {
        let piece = card
            .decoding
            .pieces
            .as_ref()
            .and_then(|p| p.get(id as usize))
            .cloned()
            .unwrap_or_default();
        let fr = token_frames.get(i).copied().unwrap_or(0);
        if piece.starts_with('▁') && !cur.is_empty() {
            flush(&mut cur, start_f, last_f, &mut words);
            start_f = fr;
        }
        if cur.is_empty() {
            start_f = fr;
        }
        cur.push_str(&piece);
        last_f = fr;
    }
    flush(&mut cur, start_f, last_f, &mut words);
    words
}

fn pick_device(options: &GigaLoadOptions) -> Result<Device, ModelError> {
    use crate::config::resolve::Device as Dev;
    match options.device {
        Dev::Cpu => Ok(Device::Cpu),
        #[cfg(not(target_os = "macos"))]
        Dev::Cuda => Device::new_cuda(0).map_err(|e| ModelError::Load(e.to_string())),
        #[cfg(all(target_os = "macos", feature = "metal"))]
        Dev::Metal => Device::new_metal(0).map_err(|e| ModelError::Load(e.to_string())),
        Dev::Auto => {
            #[cfg(all(target_os = "macos", feature = "metal"))]
            {
                if let Ok(d) = Device::new_metal(0) {
                    return Ok(d);
                }
            }
            #[cfg(not(target_os = "macos"))]
            {
                if let Ok(d) = Device::new_cuda(0) {
                    return Ok(d);
                }
            }
            Ok(Device::Cpu)
        }
    }
}
