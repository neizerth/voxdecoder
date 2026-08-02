//! Media providers (DSP / media backends).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::filter::FilterSpec;
use super::PreprocessError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaProviderSpec {
    pub name: String,
}

pub trait MediaProvider {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn apply(
        &self,
        filter: &FilterSpec,
        input: &Path,
        output: &Path,
    ) -> Result<(), PreprocessError>;
}

pub fn resolve_provider(name: &str) -> Result<Box<dyn MediaProvider>, PreprocessError> {
    match name {
        "stub" => Ok(Box::new(StubProvider)),
        "ffmpeg" => Ok(Box::new(FfmpegProvider {
            bin: find_ffmpeg()?,
        })),
        "sox" | "deepfilternet" | "rnnoise" | "demucs" => Err(PreprocessError::Unavailable(
            format!("provider '{name}' is not wired in this build; use stub or ffmpeg"),
        )),
        other => Err(PreprocessError::Usage(format!(
            "unknown media provider: {other}"
        ))),
    }
}

/// Plan-time check: stub always ok; ffmpeg ok if binary exists (or plan with argv only).
pub fn provider_available_for_plan(name: &str) -> Result<(), PreprocessError> {
    match name {
        "stub" => Ok(()),
        "ffmpeg" => {
            let _ = find_ffmpeg()?;
            Ok(())
        }
        "sox" | "deepfilternet" | "rnnoise" | "demucs" => Err(PreprocessError::Unavailable(
            format!("provider '{name}' is not wired in this build"),
        )),
        other => Err(PreprocessError::Usage(format!(
            "unknown media provider: {other}"
        ))),
    }
}

fn find_ffmpeg() -> Result<PathBuf, PreprocessError> {
    if let Ok(p) = std::env::var("VD_FFMPEG") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }
    which("ffmpeg").ok_or_else(|| {
        PreprocessError::Unavailable(
            "ffmpeg not found on PATH (set VD_FFMPEG or use --provider stub)".into(),
        )
    })
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(bin);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{bin}.exe"));
                if exe.is_file() {
                    return Some(exe);
                }
            }
        }
        None
    })
}

struct StubProvider;

impl MediaProvider for StubProvider {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn apply(
        &self,
        filter: &FilterSpec,
        input: &Path,
        output: &Path,
    ) -> Result<(), PreprocessError> {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| PreprocessError::Other(e.to_string()))?;
        }
        // Copy bytes; annotate with a tiny sidecar marker in extended attrs? Keep simple: copy.
        fs::copy(input, output).map_err(|e| {
            PreprocessError::Other(format!(
                "stub {} {}→{}: {e}",
                filter.operation,
                input.display(),
                output.display()
            ))
        })?;
        Ok(())
    }
}

struct FfmpegProvider {
    bin: PathBuf,
}

impl MediaProvider for FfmpegProvider {
    fn name(&self) -> &'static str {
        "ffmpeg"
    }

    fn apply(
        &self,
        filter: &FilterSpec,
        input: &Path,
        output: &Path,
    ) -> Result<(), PreprocessError> {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).map_err(|e| PreprocessError::Other(e.to_string()))?;
        }
        let args = ffmpeg_args(filter, input, output)?;
        let status = Command::new(&self.bin)
            .args(&args)
            .status()
            .map_err(|e| PreprocessError::Other(format!("ffmpeg spawn: {e}")))?;
        if !status.success() {
            return Err(PreprocessError::Other(format!(
                "ffmpeg failed ({status}) for operation {}",
                filter.operation
            )));
        }
        Ok(())
    }
}

pub fn ffmpeg_argv_for_plan(
    filter: &FilterSpec,
    input: &Path,
    output: &Path,
) -> Result<Vec<String>, PreprocessError> {
    ffmpeg_args(filter, input, output)
}

fn ffmpeg_args(
    filter: &FilterSpec,
    input: &Path,
    output: &Path,
) -> Result<Vec<String>, PreprocessError> {
    let mut args = vec![
        "-y".into(),
        "-i".into(),
        input.display().to_string(),
    ];
    match filter.operation.as_str() {
        "extract-audio" => {
            args.extend(["-vn".into(), "-acodec".into(), "pcm_s16le".into()]);
        }
        "convert" => {
            // container/codec from extension; leave ffmpeg defaults
        }
        "resample" => {
            let rate = param_u32(filter, "rate").unwrap_or(16_000);
            args.extend(["-ar".into(), rate.to_string()]);
        }
        "mono" => {
            args.extend(["-ac".into(), "1".into()]);
        }
        "stereo" => {
            args.extend(["-ac".into(), "2".into()]);
        }
        "normalize" => {
            args.extend(["-af".into(), "loudnorm".into()]);
        }
        "denoise" | "enhance" => {
            args.extend(["-af".into(), "afftdn".into()]);
        }
        "highpass" => {
            let hz = param_u32(filter, "cutoff_hz").unwrap_or(80);
            args.extend(["-af".into(), format!("highpass=f={hz}")]);
        }
        "lowpass" => {
            let hz = param_u32(filter, "cutoff_hz").unwrap_or(8000);
            args.extend(["-af".into(), format!("lowpass=f={hz}")]);
        }
        "compressor" => {
            args.extend(["-af".into(), "acompressor".into()]);
        }
        "speed" => {
            let factor = param_f64(filter, "factor").unwrap_or(1.0);
            if !(0.5..=2.0).contains(&factor) {
                return Err(PreprocessError::Usage(
                    "speed factor must be between 0.5 and 2.0 for atempo".into(),
                ));
            }
            args.extend(["-af".into(), format!("atempo={factor}")]);
        }
        "trim-silence" => {
            let min_d = param_str(filter, "min_duration").unwrap_or_else(|| "0.5".into());
            // silenceremove start+stop; min_duration loosely mapped
            args.extend([
                "-af".into(),
                format!("silenceremove=start_periods=1:start_silence={min_d}:stop_periods=1:stop_silence={min_d}"),
            ]);
        }
        "trim" => {
            if let Some(ss) = param_str(filter, "from").or_else(|| param_str(filter, "start")) {
                args.extend(["-ss".into(), ss]);
            }
            if let Some(to) = param_str(filter, "to").or_else(|| param_str(filter, "end")) {
                args.extend(["-to".into(), to]);
            }
        }
        "chunk" => {
            return Err(PreprocessError::Usage(
                "chunk is reserved (multi-output); not implemented yet".into(),
            ));
        }
        "split-channels" | "merge-channels" => {
            return Err(PreprocessError::Usage(format!(
                "{} not implemented in ffmpeg backend yet",
                filter.operation
            )));
        }
        other => {
            return Err(PreprocessError::Usage(format!(
                "ffmpeg does not support operation: {other}"
            )));
        }
    }
    args.push(output.display().to_string());
    Ok(args)
}

fn param_str(filter: &FilterSpec, key: &str) -> Option<String> {
    filter.params.get(key).and_then(|v| match v {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn param_u32(filter: &FilterSpec, key: &str) -> Option<u32> {
    filter.params.get(key).and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_u64().map(|u| u as u32),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

fn param_f64(filter: &FilterSpec, key: &str) -> Option<f64> {
    filter.params.get(key).and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

/// Describe planned argv without requiring binary (dry-run when ffmpeg absent still useful for stub).
pub fn describe_step(
    filter: &FilterSpec,
    input: &Path,
    output: &Path,
) -> BTreeMap<String, serde_json::Value> {
    let mut m = BTreeMap::new();
    m.insert("provider".into(), serde_json::json!(filter.provider));
    m.insert("operation".into(), serde_json::json!(filter.operation));
    m.insert("input".into(), serde_json::json!(input.display().to_string()));
    m.insert(
        "output".into(),
        serde_json::json!(output.display().to_string()),
    );
    if !filter.params.is_empty() {
        m.insert(
            "params".into(),
            serde_json::Value::Object(filter.params.clone().into_iter().collect()),
        );
    }
    if filter.provider == "ffmpeg" {
        if let Ok(argv) = ffmpeg_argv_for_plan(filter, input, output) {
            m.insert("argv".into(), serde_json::json!(argv));
        }
    }
    m
}
