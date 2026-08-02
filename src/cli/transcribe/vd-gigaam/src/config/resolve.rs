//! CLI > config > default → resolved settings.

use std::path::PathBuf;

use serde::Serialize;

use super::FileConfig;
use crate::gigaam::catalog::resolve_model_name;
use crate::output::{resolve_output_paths, OutputPathError, OutputPathRequest, OutputPaths};
use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    Cpu,
    #[cfg(not(target_os = "macos"))]
    Cuda,
    #[cfg(all(target_os = "macos", feature = "metal"))]
    Metal,
    Auto,
}

impl Device {
    /// Devices shown in help and accepted by `--device` / `config set device`.
    pub fn allowed() -> &'static [&'static str] {
        &[
            "cpu",
            #[cfg(not(target_os = "macos"))]
            "cuda",
            #[cfg(all(target_os = "macos", feature = "metal"))]
            "metal",
            "auto",
        ]
    }

    pub fn help_text() -> String {
        format!(
            "Inference device [{}] (default: auto)",
            Self::allowed().join("|")
        )
    }

    pub fn parse(s: &str) -> Option<Self> {
        if !Self::allowed().contains(&s) {
            return None;
        }
        match s {
            "cpu" => Some(Self::Cpu),
            "auto" => Some(Self::Auto),
            #[cfg(not(target_os = "macos"))]
            "cuda" => Some(Self::Cuda),
            #[cfg(all(target_os = "macos", feature = "metal"))]
            "metal" => Some(Self::Metal),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            #[cfg(not(target_os = "macos"))]
            Self::Cuda => "cuda",
            #[cfg(all(target_os = "macos", feature = "metal"))]
            Self::Metal => "metal",
            Self::Auto => "auto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Txt,
    Json,
    Srt,
    Vtt,
}

impl OutputFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "txt" => Some(Self::Txt),
            "json" => Some(Self::Json),
            "srt" => Some(Self::Srt),
            "vtt" => Some(Self::Vtt),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Json => "json",
            Self::Srt => "srt",
            Self::Vtt => "vtt",
        }
    }

    pub fn extension(self) -> &'static str {
        self.as_str()
    }
}

#[derive(Debug, Clone)]
pub struct Defaults {
    pub model: String,
    pub device: Device,
    pub fp16_encoder: bool,
    pub flash: bool,
    pub word_timestamps: bool,
    pub format: OutputFormat,
}

pub fn defaults() -> Defaults {
    Defaults {
        model: "v3_e2e_ctc".into(),
        device: Device::Auto,
        fp16_encoder: true,
        flash: false,
        word_timestamps: false,
        format: OutputFormat::Txt,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DryRunPlan {
    pub model: String,
    pub device: Device,
    pub flash: bool,
    pub fp16_encoder: bool,
    pub download_root: PathBuf,
    pub output: PathBuf,
    pub segments: Option<PathBuf>,
    pub overwrite: bool,
    pub word_timestamps: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedRun {
    pub input: PathBuf,
    pub plan: DryRunPlan,
    pub format: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct RunOverrides {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub format: Option<OutputFormat>,
    pub segments: bool,
    pub overwrite: bool,
    pub model: Option<String>,
    pub device: Option<Device>,
    pub no_fp16_encoder: bool,
    pub flash: bool,
    pub download_root: Option<PathBuf>,
    pub word_timestamps: bool,
}

pub fn resolve_run(
    file: &FileConfig,
    ov: RunOverrides,
) -> Result<ResolvedRun, OutputPathError> {
    let d = defaults();
    let model_raw = ov
        .model
        .or_else(|| file.model.clone())
        .unwrap_or_else(|| d.model.clone());
    let model = if looks_like_path(&model_raw) {
        model_raw
    } else {
        resolve_model_name(&model_raw).to_string()
    };

    let device = ov.device.or(file.device).unwrap_or(d.device);
    let flash = if crate::platform::FLASH_SUPPORTED {
        ov.flash || file.flash.unwrap_or(d.flash)
    } else {
        false
    };
    let fp16_encoder = if ov.no_fp16_encoder {
        false
    } else {
        file.fp16_encoder.unwrap_or(d.fp16_encoder)
    };
    let download_root = ov
        .download_root
        .or_else(|| {
            file.download_root
                .as_ref()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
        })
        .unwrap_or_else(paths::default_models_dir);

    let format = ov.format.or(file.format).unwrap_or(d.format);
    let word_timestamps = ov.word_timestamps || file.word_timestamps.unwrap_or(d.word_timestamps);

    let OutputPaths { main, segments } = resolve_output_paths(OutputPathRequest {
        input: ov.input.clone(),
        output: ov.output,
        output_dir: ov.output_dir,
        format,
        segments: ov.segments,
        overwrite: ov.overwrite,
    })?;

    Ok(ResolvedRun {
        input: ov.input,
        format,
        plan: DryRunPlan {
            model,
            device,
            flash,
            fp16_encoder,
            download_root,
            output: main,
            segments,
            overwrite: ov.overwrite,
            word_timestamps,
        },
    })
}

fn looks_like_path(s: &str) -> bool {
    s.contains('/') || s.contains('\\') || s.ends_with(".ckpt") || s.ends_with(".pt")
}
