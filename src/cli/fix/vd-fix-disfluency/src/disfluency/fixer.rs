//! Public `DisfluencyFixer` API.

use super::rules::{self, Mode};
use crate::types::{FixOptions, FixResult, Language, TextSpan};

#[derive(Debug, thiserror::Error)]
pub enum DisfluencyError {
    #[error("{0}")]
    Process(String),
}

impl DisfluencyError {
    pub fn exit_code(&self) -> u8 {
        1
    }
}

/// Load-time options for `DisfluencyFixer`.
#[derive(Debug, Clone, Copy)]
pub struct DisfluencyLoadOptions {
    pub language: Language,
    /// Effective mode — callers fold `remove_fillers` into this (see
    /// `config::resolve_run`): `remove_fillers = false` resolves to `Mode::Off`.
    pub mode: Mode,
}

#[derive(Debug)]
pub struct DisfluencyFixer {
    language: Language,
    mode: Mode,
}

impl DisfluencyFixer {
    /// Deterministic, table-driven — nothing to load from disk, so this
    /// cannot fail today. Kept fallible (mirrors `AsrFixer` / `CasingFixer`)
    /// so a future rule source (e.g. `--context` filler lists) doesn't need
    /// a signature break.
    pub fn load(opts: DisfluencyLoadOptions) -> Result<Self, DisfluencyError> {
        Ok(Self {
            language: opts.language,
            mode: opts.mode,
        })
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn fix(&self, span: &TextSpan<'_>, opts: FixOptions) -> Result<FixResult, DisfluencyError> {
        self.fix_text(span.text, opts)
    }

    /// Convenience for tests without a live `TextSpan`.
    pub fn fix_text(&self, text: &str, _opts: FixOptions) -> Result<FixResult, DisfluencyError> {
        Ok(rules::fix_text(text, self.language, self.mode))
    }
}
