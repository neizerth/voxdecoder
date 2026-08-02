//! CLI > config > default → resolved run plan.

use std::path::PathBuf;

use serde::Serialize;

use super::{defaults, FileConfig};
use crate::layout::language::{detect_language, resolve_language};
use crate::layout::timemap::{bind_timemap, BoundTimeMap};
use crate::models;
use crate::output::{resolve_output_path, OutputPathError, OutputPathRequest, OutputPaths};
use crate::types::{
    ArtifactType, Language, ParagraphDensity, ProgressFormat, TimeMapSource,
};

#[derive(Debug, Clone)]
pub struct RunOverrides {
    pub language: Option<Language>,
    pub density: Option<ParagraphDensity>,
    pub use_timemap: Option<bool>,
    pub timemap_cli: Option<PathBuf>,
    pub in_place: Option<bool>,
    pub progress: Option<ProgressFormat>,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub overwrite: bool,
    pub cli_in_place: bool,
    pub download_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ResolvedRun {
    pub input: PathBuf,
    pub artifact_type: ArtifactType,
    pub language_requested: Language,
    pub language: Language,
    pub model: String,
    pub models_dir: PathBuf,
    pub installed: bool,
    pub density: ParagraphDensity,
    pub use_timemap: bool,
    pub timemap: BoundTimeMap,
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
    pub language_resolved: String,
    pub model: String,
    pub installed: bool,
    pub density: String,
    pub timemap: Option<DryRunTimeMap>,
    pub in_place: bool,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunTimeMap {
    pub source: String,
}

impl ResolvedRun {
    pub fn dry_run_plan(&self) -> DryRunPlan {
        let timemap = match self.timemap.source {
            TimeMapSource::None => None,
            other => Some(DryRunTimeMap {
                source: other.as_str().to_string(),
            }),
        };
        DryRunPlan {
            input: self.input.display().to_string(),
            artifact_type: self.artifact_type.as_str().to_string(),
            output: self.paths.main.display().to_string(),
            language: self.language_requested.as_str().to_string(),
            language_resolved: self.language.as_str().to_string(),
            model: self.model.clone(),
            installed: self.installed,
            density: self.density.as_str().to_string(),
            timemap,
            in_place: self.paths.in_place,
            overwrite: self.overwrite,
        }
    }

    pub fn dry_run_text(&self) -> String {
        let plan = self.dry_run_plan();
        let tm = plan.timemap.as_ref().map_or_else(
            || "TimeMap:\n  source: none".to_string(),
            |t| format!("TimeMap:\n  source: {}", t.source),
        );
        format!(
            "Input: {}\nArtifact type: {}\nOutput: {}\nLanguage: {}\nLanguage resolved: {}\nModel: {}\nPack installed: {}\nDensity: {}\n{}\nIn-place: {}\nOverwrite: {}",
            plan.input,
            plan.artifact_type,
            plan.output,
            plan.language,
            plan.language_resolved,
            plan.model,
            if plan.installed { "yes" } else { "no (builtin)" },
            plan.density,
            tm,
            if plan.in_place { "on" } else { "off" },
            if plan.overwrite { "on" } else { "off" },
        )
    }
}

pub fn resolve_run(
    input: PathBuf,
    artifact_type: ArtifactType,
    sample_text: &str,
    file: &FileConfig,
    ov: RunOverrides,
) -> Result<ResolvedRun, OutputPathError> {
    let d = defaults();
    let language_requested = ov.language.or(file.language).unwrap_or(d.language);
    let density = ov
        .density
        .or(file.paragraph_density)
        .unwrap_or(d.paragraph_density);
    let use_timemap = ov.use_timemap.or(file.use_timemap).unwrap_or(d.use_timemap);

    let timemap = if use_timemap {
        bind_timemap(&input, ov.timemap_cli.as_deref())
    } else {
        BoundTimeMap {
            source: TimeMapSource::None,
            map: None,
        }
    };

    let language = resolve_language(language_requested, sample_text, file.language.or(Some(d.language)));
    // If still Auto somehow, fall back via detect then ru.
    let language = match language {
        Language::Auto => detect_language(sample_text).unwrap_or(Language::Ru),
        other => other,
    };

    let model = models::resolve_model_name(language.as_str()).to_string();
    let models_dir = crate::paths::resolve_models_dir(ov.download_root.clone());
    let installed = models::is_installed(&models_dir, &model);

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

    Ok(ResolvedRun {
        input,
        artifact_type,
        language_requested,
        language,
        model,
        models_dir,
        installed,
        density,
        use_timemap,
        timemap,
        paths,
        overwrite: ov.overwrite || in_place,
        progress,
    })
}
