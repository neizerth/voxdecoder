//! Public `AsrFixer` API.

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use super::config::AsrLoadOptions;
use super::context_fuzzy;
use super::lang;
use super::rule::RuleHit;
use super::stages::{
    alphabet, dictionary, duplicate, merge_split, punctuation, spacing, ConfidencePolicy, Pipeline,
};
use crate::context::{load_materials, Materials, SpanContext};
use crate::types::{FixOptions, FixResult, TextSpan};

/// `AsrFixer::fix`/`fix_text` result: the rewritten text plus every rule hit
/// found along the way (for `--report`), including ones a strict policy
/// declined to apply. Derefs to `FixResult` so existing `.text`/`.changed`
/// call sites keep working unchanged.
#[derive(Debug)]
pub struct FixOutcome {
    pub result: FixResult,
    pub hits: Vec<RuleHit>,
}

impl Deref for FixOutcome {
    type Target = FixResult;
    fn deref(&self) -> &FixResult {
        &self.result
    }
}

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
    materials: Materials,
    dictionary: Arc<HashMap<String, String>>,
    policy: ConfidencePolicy,
    #[allow(dead_code)]
    neighbor_window: u32,
}

impl AsrFixer {
    pub fn load(opts: AsrLoadOptions) -> Result<Self, AsrError> {
        let materials = load_materials(&opts.context_paths).map_err(AsrError::BackendInit)?;
        let dictionary = lang::resolve_dictionary(
            opts.language,
            &opts.dictionary_paths,
            opts.project_dir.as_deref(),
        );
        Ok(Self {
            materials,
            dictionary: Arc::new(dictionary),
            policy: opts.confidence_policy,
            neighbor_window: opts.neighbor_window,
        })
    }

    pub fn materials(&self) -> &Materials {
        &self.materials
    }

    pub fn neighbor_window(&self) -> u32 {
        self.neighbor_window
    }

    /// Stages 1–6 (static half) of ADR 0010. The fuzzy half of Stage 6
    /// needs `SpanContext` and runs separately in `fix_text` below.
    fn cleanup_pipeline(&self) -> Pipeline {
        Pipeline::new(vec![
            Box::new(spacing::stage()),
            Box::new(punctuation::stage()),
            Box::new(duplicate::stage()),
            Box::new(merge_split::stage()),
            Box::new(alphabet::stage()),
            Box::new(dictionary::stage(Arc::clone(&self.dictionary))),
        ])
    }

    pub fn fix(
        &self,
        span: &TextSpan<'_>,
        ctx: SpanContext<'_>,
        opts: FixOptions,
    ) -> Result<FixOutcome, AsrError> {
        self.fix_text(span.text, ctx, opts)
    }

    /// Convenience for tests without a live `TextSpan`.
    pub fn fix_text(
        &self,
        text: &str,
        ctx: SpanContext<'_>,
        _opts: FixOptions,
    ) -> Result<FixOutcome, AsrError> {
        let staged = self.cleanup_pipeline().run(text, &self.policy);
        let (out, fuzzy_hits) = context_fuzzy::apply(&staged.text, ctx, self.policy);
        let mut hits = staged.hits;
        hits.extend(fuzzy_hits);
        Ok(FixOutcome {
            result: FixResult {
                changed: out != text,
                text: out,
            },
            hits,
        })
    }
}
