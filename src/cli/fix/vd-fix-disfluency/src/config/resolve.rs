//! CLI > config > default → resolved run plan.

use std::path::PathBuf;

use serde::Serialize;

use super::{defaults, FileConfig};
use crate::output::{resolve_output_path, OutputPathError, OutputPathRequest, OutputPaths};
use crate::types::{ArtifactType, Language, Mode, ProgressFormat};

#[derive(Debug, Clone)]
pub struct RunOverrides {
    pub language: Option<Language>,
    pub mode: Option<Mode>,
    pub remove_fillers: Option<bool>,
    pub in_place: Option<bool>,
    pub progress: Option<ProgressFormat>,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub overwrite: bool,
    pub cli_in_place: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedRun {
    pub input: PathBuf,
    pub artifact_type: ArtifactType,
    pub language: Language,
    /// Effective mode already folds in `remove_fillers = false` → `Mode::Off`.
    pub mode: Mode,
    pub remove_fillers: bool,
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
    pub mode: String,
    pub remove_fillers: bool,
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
            mode: self.mode.as_str().to_string(),
            remove_fillers: self.remove_fillers,
            in_place: self.paths.in_place,
            overwrite: self.overwrite,
        }
    }

    pub fn dry_run_text(&self) -> String {
        let plan = self.dry_run_plan();
        format!(
            "Input: {}\nArtifact type: {}\nOutput: {}\nLanguage: {}\nMode: {}\nRemove fillers: {}\nIn-place: {}\nOverwrite: {}",
            plan.input,
            plan.artifact_type,
            plan.output,
            plan.language,
            plan.mode,
            if plan.remove_fillers { "on" } else { "off" },
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
    let configured_mode = ov.mode.or(file.mode).unwrap_or(d.mode);
    let remove_fillers = ov
        .remove_fillers
        .or(file.remove_fillers)
        .unwrap_or(d.remove_fillers);
    let mode = if remove_fillers {
        configured_mode
    } else {
        Mode::Off
    };

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
        language,
        mode,
        remove_fillers,
        paths,
        overwrite: ov.overwrite || in_place,
        progress,
    })
}
