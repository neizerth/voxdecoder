//! CLI > config > default → resolved run plan.

use std::path::PathBuf;

use serde::Serialize;

use super::{defaults, FileConfig};
use crate::output::{resolve_output_path, OutputPathError, OutputPathRequest, OutputPaths};
use crate::types::{ArtifactType, Language, ProgressFormat};

#[derive(Debug, Clone)]
pub struct RunOverrides {
    pub language: Option<Language>,
    pub in_place: Option<bool>,
    pub progress: Option<ProgressFormat>,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub overwrite: bool,
    pub cli_in_place: bool,
    pub terms: Vec<PathBuf>,
    pub shipping: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedRun {
    pub input: PathBuf,
    pub artifact_type: ArtifactType,
    pub language: Language,
    pub terms: Vec<PathBuf>,
    pub shipping: bool,
    pub paths: OutputPaths,
    pub overwrite: bool,
    pub progress: ProgressFormat,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunPlan {
    pub input: String,
    pub artifact_type: String,
    pub output: String,
    pub language: String,
    pub terms: Vec<String>,
    pub shipping_lexicon: bool,
    pub in_place: bool,
    pub overwrite: bool,
}

impl ResolvedRun {
    pub fn dry_run_plan(&self) -> DryRunPlan {
        DryRunPlan {
            input: self.input.display().to_string(),
            artifact_type: self.artifact_type.as_str().to_string(),
            output: self.paths.main.display().to_string(),
            language: self.language.as_str().to_string(),
            terms: self.terms.iter().map(|p| p.display().to_string()).collect(),
            shipping_lexicon: self.shipping,
            in_place: self.paths.in_place,
            overwrite: self.overwrite,
        }
    }

    pub fn dry_run_text(&self) -> String {
        let plan = self.dry_run_plan();
        let terms = if plan.terms.is_empty() {
            "—".to_string()
        } else {
            plan.terms.join(", ")
        };
        format!(
            "Input: {}\nArtifact type: {}\nOutput: {}\nLanguage: {}\nTerms: {}\nShipping lexicon: {}\nIn-place: {}\nOverwrite: {}",
            plan.input,
            plan.artifact_type,
            plan.output,
            plan.language,
            terms,
            if plan.shipping_lexicon { "yes" } else { "no" },
            if plan.in_place { "on" } else { "off" },
            if plan.overwrite { "on" } else { "off" },
        )
    }
}

pub fn resolve_run(
    input: PathBuf,
    artifact_type: ArtifactType,
    file: &FileConfig,
    ov: RunOverrides,
) -> Result<ResolvedRun, OutputPathError> {
    let d = defaults();
    let language = ov.language.or(file.language).unwrap_or(d.language);

    let in_place = if ov.cli_in_place {
        true
    } else if ov.output.is_some() || ov.output_dir.is_some() {
        false
    } else {
        ov.in_place.or(file.in_place).unwrap_or(d.in_place)
    };
    let progress = ov.progress.or(file.progress).unwrap_or(d.progress);

    let paths = resolve_output_path(OutputPathRequest {
        input: input.clone(),
        output: ov.output,
        output_dir: ov.output_dir,
        in_place,
        overwrite: ov.overwrite || in_place,
        default_file_name: crate::output::fixed_file_name(&input, artifact_type.extension()),
    })?;

    let terms = if ov.terms.is_empty() {
        vd_artifact::paths::project_dir_if_present(&input)
            .into_iter()
            .collect()
    } else {
        ov.terms
    };

    Ok(ResolvedRun {
        input,
        artifact_type,
        language,
        terms,
        shipping: ov.shipping,
        paths,
        overwrite: ov.overwrite || in_place,
        progress,
    })
}
