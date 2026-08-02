//! Public `LayoutFixer` API.

use super::backend;
use super::config::LayoutLoadOptions;
use crate::models::{self, Lexicon};
use crate::types::{FixOptions, FixResult, Language, ParagraphDensity, TimeMap};

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("inference backend failed to initialize: {0}")]
    BackendInit(String),
    #[error("{0}")]
    Process(String),
}

impl LayoutError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::BackendInit(_) => 4,
            Self::Process(_) => 1,
        }
    }
}

#[derive(Debug)]
pub struct LayoutFixer {
    language: Language,
    density: ParagraphDensity,
    lexicon: Lexicon,
    timemap: Option<TimeMap>,
}

impl LayoutFixer {
    pub fn load(opts: LayoutLoadOptions) -> Result<Self, LayoutError> {
        let language = match opts.language {
            Language::Auto => Language::Ru,
            other => other,
        };
        let lexicon = models::resolve_lexicon(&opts.models_dir, language)
            .map_err(|e| LayoutError::BackendInit(e.to_string()))?;
        Ok(Self {
            language,
            density: opts.density,
            lexicon,
            timemap: if opts.use_timemap {
                opts.timemap
            } else {
                None
            },
        })
    }

    pub fn fix(&self, text: &str, _opts: FixOptions) -> Result<FixResult, LayoutError> {
        let out = backend::rewrite(
            text,
            self.language,
            self.density,
            &self.lexicon,
            self.timemap.as_ref(),
        );
        Ok(FixResult {
            changed: out != text,
            text: out,
        })
    }
}
