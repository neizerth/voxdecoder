//! Public `TermsFixer` API.

use super::backend;
use crate::lexicon::Lexicon;
use crate::types::FixResult;

#[derive(Debug, thiserror::Error)]
pub enum TermsError {
    #[error("lexicon backend failed to initialize: {0}")]
    BackendInit(String),
    #[error("{0}")]
    Process(String),
}

impl TermsError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::BackendInit(_) => 4,
            Self::Process(_) => 1,
        }
    }
}

#[derive(Debug)]
pub struct TermsFixer {
    lexicon: Lexicon,
}

impl TermsFixer {
    pub fn new(lexicon: Lexicon) -> Result<Self, TermsError> {
        Ok(Self { lexicon })
    }

    pub fn fix(&self, text: &str) -> Result<FixResult, TermsError> {
        let out = backend::rewrite(text, &self.lexicon);
        Ok(FixResult {
            changed: out != text,
            text: out,
        })
    }

    pub fn lexicon(&self) -> &Lexicon {
        &self.lexicon
    }
}
