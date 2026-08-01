//! Public `CasingFixer` API.

use super::backend;
use super::config::CasingLoadOptions;
use crate::models::{self, Lexicon};
use crate::types::{FixOptions, FixResult, Language};

#[derive(Debug, thiserror::Error)]
pub enum CasingError {
    #[error("inference backend failed to initialize: {0}")]
    BackendInit(String),
    #[error("{0}")]
    Process(String),
}

impl CasingError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::BackendInit(_) => 4,
            Self::Process(_) => 1,
        }
    }
}

#[derive(Debug)]
pub struct CasingFixer {
    language: Language,
    lexicon: Lexicon,
}

impl CasingFixer {
    pub fn load(opts: CasingLoadOptions) -> Result<Self, CasingError> {
        let lexicon = models::resolve_lexicon(&opts.models_dir, opts.language)
            .map_err(|e| CasingError::BackendInit(e.to_string()))?;
        Ok(Self {
            language: opts.language,
            lexicon,
        })
    }

    pub fn fix(&self, text: &str, _opts: FixOptions) -> Result<FixResult, CasingError> {
        let out = backend::rewrite(text, self.language, &self.lexicon);
        Ok(FixResult {
            changed: out != text,
            text: out,
        })
    }
}
