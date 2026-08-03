//! Linguistic infrastructure adapters for Natasha/razdel (vd-text-py sidecar).
//!
//! Bridges Rust to Python sidecar for tokenization, sentence segmentation, morphology.
//! Long-lived subprocess, graceful degradation if unavailable.

use serde::{Deserialize, Serialize};
use std::process::Command;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LinguisticsError {
    #[error("Sidecar not available: {0}")]
    SidecarUnavailable(String),
    #[error("Sidecar error: {0}")]
    SidecarError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::error::Error),
}

/// Token with byte offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Sentence with byte offsets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentence {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Morphological analysis for a word.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Morph {
    pub text: String,
    pub grammemes: Vec<String>,
    pub normalized: String,
    pub pos: Option<String>,
}

/// Tokenizer backed by vd-text-py sidecar.
pub struct Tokenizer {
    sidecar_bin: String,
}

impl Tokenizer {
    /// Create tokenizer. Sidecar binary name defaults to "vd-text-py".
    pub fn new() -> Self {
        Self {
            sidecar_bin: "vd-text-py".to_string(),
        }
    }

    /// Tokenize text by invoking sidecar.
    pub fn tokenize(&self, text: &str) -> Result<Vec<Token>, LinguisticsError> {
        let temp_dir = std::env::temp_dir();
        let input_file = temp_dir.join(format!("vd-text-tokenize-{}.txt", uuid_v4()));
        let output_file = temp_dir.join(format!("vd-text-tokenize-{}.json", uuid_v4()));

        // Write input
        std::fs::write(&input_file, text)?;

        // Invoke sidecar
        let status = Command::new(&self.sidecar_bin)
            .arg("-i")
            .arg(&input_file)
            .arg("-o")
            .arg(&output_file)
            .arg("-op")
            .arg("tokenize")
            .status()?;

        // Check for errors
        if !status.success() {
            let _ = std::fs::remove_file(&input_file);
            let _ = std::fs::remove_file(&output_file);
            return Err(LinguisticsError::SidecarError(
                "tokenize operation failed".to_string(),
            ));
        }

        // Parse output
        let output_text = std::fs::read_to_string(&output_file)?;
        let response: TokenizeResponse = serde_json::from_str(&output_text)?;

        // Cleanup
        let _ = std::fs::remove_file(&input_file);
        let _ = std::fs::remove_file(&output_file);

        if let Some(error) = response.error {
            return Err(LinguisticsError::SidecarError(error));
        }

        Ok(response.tokens)
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct TokenizeResponse {
    operation: String,
    tokens: Vec<Token>,
    error: Option<String>,
}

/// Sentence splitter backed by vd-text-py sidecar.
pub struct SentenceSplitter {
    sidecar_bin: String,
}

impl SentenceSplitter {
    /// Create sentence splitter.
    pub fn new() -> Self {
        Self {
            sidecar_bin: "vd-text-py".to_string(),
        }
    }

    /// Split text into sentences.
    pub fn split(&self, text: &str) -> Result<Vec<Sentence>, LinguisticsError> {
        let temp_dir = std::env::temp_dir();
        let input_file = temp_dir.join(format!("vd-text-sentenize-{}.txt", uuid_v4()));
        let output_file = temp_dir.join(format!("vd-text-sentenize-{}.json", uuid_v4()));

        // Write input
        std::fs::write(&input_file, text)?;

        // Invoke sidecar
        let status = Command::new(&self.sidecar_bin)
            .arg("-i")
            .arg(&input_file)
            .arg("-o")
            .arg(&output_file)
            .arg("-op")
            .arg("sentence_split")
            .status()?;

        if !status.success() {
            let _ = std::fs::remove_file(&input_file);
            let _ = std::fs::remove_file(&output_file);
            return Err(LinguisticsError::SidecarError(
                "sentence_split operation failed".to_string(),
            ));
        }

        let output_text = std::fs::read_to_string(&output_file)?;
        let response: SentenceResponse = serde_json::from_str(&output_text)?;

        let _ = std::fs::remove_file(&input_file);
        let _ = std::fs::remove_file(&output_file);

        if let Some(error) = response.error {
            return Err(LinguisticsError::SidecarError(error));
        }

        Ok(response.sentences)
    }
}

impl Default for SentenceSplitter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct SentenceResponse {
    operation: String,
    sentences: Vec<Sentence>,
    error: Option<String>,
}

/// Morphological analyzer backed by vd-text-py sidecar.
pub struct Morphology {
    sidecar_bin: String,
}

impl Morphology {
    /// Create morphology analyzer.
    pub fn new() -> Self {
        Self {
            sidecar_bin: "vd-text-py".to_string(),
        }
    }

    /// Analyze text morphologically (word-by-word).
    pub fn analyze(&self, text: &str) -> Result<Vec<Morph>, LinguisticsError> {
        let temp_dir = std::env::temp_dir();
        let input_file = temp_dir.join(format!("vd-text-morph-{}.txt", uuid_v4()));
        let output_file = temp_dir.join(format!("vd-text-morph-{}.json", uuid_v4()));

        std::fs::write(&input_file, text)?;

        let status = Command::new(&self.sidecar_bin)
            .arg("-i")
            .arg(&input_file)
            .arg("-o")
            .arg(&output_file)
            .arg("-op")
            .arg("morph")
            .status()?;

        if !status.success() {
            let _ = std::fs::remove_file(&input_file);
            let _ = std::fs::remove_file(&output_file);
            return Err(LinguisticsError::SidecarError(
                "morph operation failed".to_string(),
            ));
        }

        let output_text = std::fs::read_to_string(&output_file)?;
        let response: MorphResponse = serde_json::from_str(&output_text)?;

        let _ = std::fs::remove_file(&input_file);
        let _ = std::fs::remove_file(&output_file);

        if let Some(error) = response.error {
            return Err(LinguisticsError::SidecarError(error));
        }

        Ok(response.analyses)
    }
}

impl Default for Morphology {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct MorphResponse {
    operation: String,
    analyses: Vec<Morph>,
    error: Option<String>,
}

/// Simple unique ID generator for temp file names.
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Use time-based unique ID (sufficient for temp files)
    format!("{:x}", now)
}
