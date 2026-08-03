//! Shared helpers for vd-pipeline e2e binary tests.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::cargo::cargo_bin;

pub fn child_available(name: &str) -> bool {
    if let Ok(p) = which_near_pipeline(name) {
        return p.exists();
    }
    StdCommand::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn which_near_pipeline(name: &str) -> Result<PathBuf, ()> {
    let pipe = cargo_bin!("vd-pipeline");
    let candidate = pipe.parent().ok_or(())?.join(name);
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(())
    }
}

/// True when the sibling `vd-gigaam` binary accepts `--device metal` (Metal build).
pub fn gigaam_supports_metal() -> bool {
    let Ok(bin) = which_near_pipeline("vd-gigaam") else {
        return false;
    };
    let out = StdCommand::new(&bin).args(["run", "--help"]).output().ok();
    let Some(out) = out else {
        return false;
    };
    let help = String::from_utf8_lossy(&out.stdout);
    help.contains("metal")
}

pub fn ffmpeg_available() -> bool {
    if let Ok(p) = std::env::var("VD_FFMPEG") {
        return Path::new(&p).is_file();
    }
    StdCommand::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn gigaam_models_root() -> PathBuf {
    let models_root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../transcribe/vd-gigaam/models");
    models_root.canonicalize().unwrap_or(models_root)
}

pub fn ctc_model_ready(models_root: &Path) -> bool {
    models_root.join("v3_e2e_ctc/model.safetensors").is_file()
}

/// Prefer target-dir siblings (`vd-gigaam`, `vd-preprocess`, …) on PATH.
pub fn path_with_pipeline_siblings() -> OsString {
    let mut dirs = Vec::new();
    if let Ok(d) = which_near_pipeline("vd-gigaam") {
        if let Some(parent) = d.parent() {
            dirs.push(parent.to_path_buf());
        }
    }
    if let Ok(d) = which_near_pipeline("vd-preprocess") {
        if let Some(parent) = d.parent() {
            if !dirs.iter().any(|p| p == parent) {
                dirs.push(parent.to_path_buf());
            }
        }
    }
    let mut out = OsString::new();
    for (i, dir) in dirs.iter().enumerate() {
        if i > 0 {
            out.push(":");
        }
        out.push(dir.as_os_str());
    }
    if let Some(rest) = std::env::var_os("PATH") {
        if !out.is_empty() {
            out.push(":");
        }
        out.push(rest);
    }
    out
}

pub fn env_path_prepend(dir: &Path) -> OsString {
    let mut out = dir.as_os_str().to_owned();
    if let Some(rest) = std::env::var_os("PATH") {
        out.push(":");
        out.push(rest);
    }
    out
}

/// Fraction of reference words that appear in the ASR hypothesis (order-insensitive bag).
pub fn word_coverage(expected: &str, got: &str) -> f64 {
    let exp: Vec<String> = normalize_words(expected);
    let hyp: std::collections::HashSet<String> = normalize_words(got).into_iter().collect();
    if exp.is_empty() {
        return 1.0;
    }
    let hits = exp.iter().filter(|w| hyp.contains(*w)).count();
    hits as f64 / exp.len() as f64
}

pub fn normalize_words(s: &str) -> Vec<String> {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| if c == 'ё' { 'е' } else { c })
        .map(|c| {
            if c.is_alphanumeric() || c.is_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

pub fn report_step_ms(report: &serde_json::Value, capability: &str) -> Option<u64> {
    report["steps"].as_array()?.iter().find_map(|s| {
        if s["capability"].as_str() == Some(capability) {
            s["duration_ms"].as_u64()
        } else {
            None
        }
    })
}
