//! Public `AsrFixer` API.

use super::backend;
use super::config::AsrLoadOptions;
use crate::context::{load_materials, Materials, SpanContext};
use crate::types::{FixOptions, FixResult, Language, TextSpan};

#[derive(Debug, thiserror::Error)]
pub enum AsrError {
    #[error("inference backend failed to initialize: {0}")]
    BackendInit(String),
    #[error("{0}")]
    Process(String),
}

impl AsrError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::BackendInit(_) => 4,
            Self::Process(_) => 1,
        }
    }
}

#[derive(Debug)]
pub struct AsrFixer {
    language: Language,
    materials: Materials,
    #[allow(dead_code)]
    neighbor_window: u32,
}

impl AsrFixer {
    pub fn load(opts: AsrLoadOptions) -> Result<Self, AsrError> {
        let materials = load_materials(&opts.context_paths).map_err(AsrError::BackendInit)?;
        Ok(Self {
            language: opts.language,
            materials,
            neighbor_window: opts.neighbor_window,
        })
    }

    pub fn materials(&self) -> &Materials {
        &self.materials
    }

    pub fn neighbor_window(&self) -> u32 {
        self.neighbor_window
    }

    pub fn fix(
        &self,
        span: &TextSpan<'_>,
        ctx: SpanContext<'_>,
        _opts: FixOptions,
    ) -> Result<FixResult, AsrError> {
        let out = backend::rewrite(span.text, self.language, ctx);
        Ok(FixResult {
            changed: out != *span.text,
            text: out,
        })
    }

    /// Convenience for tests without a live `TextSpan`.
    pub fn fix_text(
        &self,
        text: &str,
        ctx: SpanContext<'_>,
        _opts: FixOptions,
    ) -> Result<FixResult, AsrError> {
        let out = backend::rewrite(text, self.language, ctx);
        Ok(FixResult {
            changed: out != text,
            text: out,
        })
    }
}
