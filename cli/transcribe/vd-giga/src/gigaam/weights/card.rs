//! Model card written by `scripts/convert_ckpt.py`.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct ModelCard {
    pub model_name: String,
    pub decoder: String,
    pub encoder: EncoderCard,
    pub head: HeadCard,
    pub preprocessor: PreprocessorCard,
    pub decoding: DecodingCard,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncoderCard {
    pub feat_in: usize,
    pub n_layers: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub ff_expansion_factor: usize,
    pub subsampling: String,
    pub subs_kernel_size: usize,
    pub subsampling_factor: usize,
    pub self_attention_model: String,
    pub conv_kernel_size: usize,
    pub conv_norm_type: String,
    pub pos_emb_max_len: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HeadCard {
    pub feat_in: usize,
    pub num_classes: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreprocessorCard {
    pub sample_rate: u32,
    pub features: usize,
    pub n_fft: usize,
    pub win_length: usize,
    pub hop_length: usize,
    pub center: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DecodingCard {
    pub vocabulary: Option<Vec<String>>,
    pub tokenizer_file: Option<String>,
    pub pieces: Option<Vec<String>>,
}

impl ModelCard {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }

    pub fn blank_id(&self) -> u32 {
        if let Some(pieces) = &self.decoding.pieces {
            pieces.len() as u32
        } else if let Some(vocab) = &self.decoding.vocabulary {
            vocab.len() as u32
        } else {
            self.head.num_classes.saturating_sub(1) as u32
        }
    }

    pub fn decode_tokens(&self, ids: &[u32]) -> String {
        let pieces: &[String] = if let Some(p) = &self.decoding.pieces {
            p
        } else if let Some(v) = &self.decoding.vocabulary {
            v
        } else {
            return String::new();
        };
        let mut raw = String::new();
        for &id in ids {
            if let Some(p) = pieces.get(id as usize) {
                raw.push_str(p);
            }
        }
        raw.replace('▁', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}
