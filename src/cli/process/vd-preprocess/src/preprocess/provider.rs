//! Media providers (DSP / media backends).

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
        "ffmpeg" => Ok(Box::new(FfmpegProvider)),
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

/// Best-effort ffmpeg path for duration probing (may be missing).
pub(crate) fn find_ffmpeg_for_probe() -> Option<PathBuf> {
    find_ffmpeg().ok()
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

struct FfmpegProvider;

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
        apply_ffmpeg(filter, input, output, None, None)
    }
}

/// Run one ffmpeg filter step, optionally reporting local 0–100 progress via `-progress pipe:1`.
pub fn apply_ffmpeg(
    filter: &FilterSpec,
    input: &Path,
    output: &Path,
    duration_sec: Option<f64>,
    on_progress: Option<&dyn Fn(u8)>,
) -> Result<(), PreprocessError> {
    let bin = find_ffmpeg()?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| PreprocessError::Other(e.to_string()))?;
    }
    let mut args = ffmpeg_args(filter, input, output)?;
    // After `-y`: quiet stats + machine-readable progress on stdout.
    args.insert(1, "-nostats".into());
    args.insert(2, "-loglevel".into());
    args.insert(3, "error".into());
    args.insert(4, "-progress".into());
    args.insert(5, "pipe:1".into());

    let mut child = Command::new(&bin)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| PreprocessError::Other(format!("ffmpeg spawn: {e}")))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| PreprocessError::Other("ffmpeg stdout missing".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| PreprocessError::Other("ffmpeg stderr missing".into()))?;

    // Drain stderr concurrently so a noisy failure cannot deadlock the pipe.
    let stderr_handle = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = BufReader::new(stderr).read_to_string(&mut buf);
        buf
    });

    let mut last_pct = 0u8;
    if let Some(cb) = on_progress {
        cb(0);
    }
    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let Ok(line) = line else {
            break;
        };
        if let Some(ms) = line.strip_prefix("out_time_ms=") {
            if let (Some(dur), Ok(ms)) = (duration_sec, ms.trim().parse::<f64>()) {
                if dur > 0.0 {
                    let pct = ((ms / 1000.0 / dur) * 100.0).clamp(0.0, 99.0) as u8;
                    if pct != last_pct {
                        last_pct = pct;
                        if let Some(cb) = on_progress {
                            cb(pct);
                        }
                    }
                }
            }
        } else if line.trim() == "progress=end" {
            if let Some(cb) = on_progress {
                cb(100);
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| PreprocessError::Other(format!("ffmpeg wait: {e}")))?;
    let err_tail = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        let detail = compact_tool_output(&err_tail);
        return Err(PreprocessError::Other(if detail.is_empty() {
            format!("ffmpeg failed ({status}) for operation {}", filter.operation)
        } else {
            format!(
                "ffmpeg failed ({status}) for operation {}: {detail}",
                filter.operation
            )
        }));
    }
    if let Some(cb) = on_progress {
        cb(100);
    }
    Ok(())
}

/// Keep a short, MCP-safe snippet: drop ffmpeg progress spam, prefer trailing lines.
pub(crate) fn compact_tool_output(raw: &str) -> String {
    const MAX: usize = 1200;
    let cleaned = raw.replace('\r', "\n");
    let mut lines: Vec<&str> = cleaned
        .lines()
        .map(str::trim)
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("ffmpeg version")
                && !l.starts_with("built with")
                && !l.starts_with("configuration:")
                && !l.starts_with("libav")
                && !l.starts_with("libsw")
                && !l.starts_with("size=")
                && !l.starts_with("frame=")
                && !l.starts_with("Press [")
                && !l.starts_with("Stream mapping:")
                && !l.starts_with("Output #")
                && !l.starts_with("Input #")
                && !l.starts_with("  Stream #")
                && !l.starts_with("  Metadata:")
                && !l.starts_with("    ")
        })
        .collect();
    // Keep the last few meaningful lines (real errors live at the end).
    if lines.len() > 12 {
        lines = lines.split_off(lines.len() - 12);
    }
    let joined = lines.join("\n");
    if joined.len() <= MAX {
        return joined;
    }
    let start = joined.len().saturating_sub(MAX);
    let clipped = &joined[start..];
    format!("…{}", clipped.trim_start())
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
            // Drop video; write PCM WAV (output path should use .wav).
            args.extend([
                "-vn".into(),
                "-acodec".into(),
                "pcm_s16le".into(),
                "-f".into(),
                "wav".into(),
            ]);
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
            let af = atempo_filtergraph(factor)?;
            args.extend(["-af".into(), af]);
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
        "pad-start" => {
            let sec = param_f64(filter, "duration_sec").ok_or_else(|| {
                PreprocessError::Usage("pad-start requires duration_sec".into())
            })?;
            if !sec.is_finite() || sec < 0.0 {
                return Err(PreprocessError::Usage(format!(
                    "pad-start duration_sec must be >= 0 (got {sec})"
                )));
            }
            let ms = (sec * 1000.0).round().max(0.0) as u64;
            args.extend(["-af".into(), format!("adelay=delays={ms}:all=1")]);
        }
        "pad-end" => {
            let sec = param_f64(filter, "duration_sec").ok_or_else(|| {
                PreprocessError::Usage("pad-end requires duration_sec".into())
            })?;
            if !sec.is_finite() || sec < 0.0 {
                return Err(PreprocessError::Usage(format!(
                    "pad-end duration_sec must be >= 0 (got {sec})"
                )));
            }
            args.extend(["-af".into(), format!("apad=pad_dur={sec}")]);
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

/// ffmpeg `atempo` accepts ~0.5..=2.0 per stage; chain stages for 0.25..=4.0.
fn atempo_filtergraph(factor: f64) -> Result<String, PreprocessError> {
    const MIN: f64 = 0.25;
    const MAX: f64 = 4.0;
    const STAGE_MIN: f64 = 0.5;
    const STAGE_MAX: f64 = 2.0;

    if !factor.is_finite() || !(MIN..=MAX).contains(&factor) {
        return Err(PreprocessError::Usage(format!(
            "speed factor must be between {MIN} and {MAX} (got {factor})"
        )));
    }
    if (factor - 1.0).abs() < 1e-9 {
        return Ok("atempo=1.0".into());
    }

    let mut remaining = factor;
    let mut stages = Vec::new();
    while remaining > STAGE_MAX + 1e-9 {
        stages.push(STAGE_MAX);
        remaining /= STAGE_MAX;
    }
    while remaining < STAGE_MIN - 1e-9 {
        stages.push(STAGE_MIN);
        remaining /= STAGE_MIN;
    }
    stages.push(remaining);

    Ok(stages
        .into_iter()
        .map(|s| format!("atempo={s}"))
        .collect::<Vec<_>>()
        .join(","))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn speed_filter(factor: f64) -> FilterSpec {
        let mut params = BTreeMap::new();
        params.insert("factor".into(), serde_json::json!(factor));
        FilterSpec {
            operation: "speed".into(),
            provider: "ffmpeg".into(),
            params,
        }
    }

    #[test]
    fn atempo_within_single_stage() {
        assert_eq!(atempo_filtergraph(1.25).unwrap(), "atempo=1.25");
        assert_eq!(atempo_filtergraph(2.0).unwrap(), "atempo=2");
    }

    #[test]
    fn atempo_chains_above_2() {
        assert_eq!(atempo_filtergraph(4.0).unwrap(), "atempo=2,atempo=2");
        assert_eq!(atempo_filtergraph(3.0).unwrap(), "atempo=2,atempo=1.5");
        assert_eq!(atempo_filtergraph(2.5).unwrap(), "atempo=2,atempo=1.25");
    }

    #[test]
    fn atempo_rejects_out_of_range() {
        assert!(atempo_filtergraph(0.1).is_err());
        assert!(atempo_filtergraph(4.1).is_err());
    }

    #[test]
    fn ffmpeg_argv_embeds_chained_atempo() {
        let argv = ffmpeg_argv_for_plan(
            &speed_filter(3.5),
            Path::new("in.wav"),
            Path::new("out.wav"),
        )
        .unwrap();
        let af = argv
            .windows(2)
            .find(|w| w[0] == "-af")
            .map(|w| w[1].as_str())
            .unwrap();
        assert_eq!(af, "atempo=2,atempo=1.75");
    }

    #[test]
    fn compact_tool_output_keeps_tail_error() {
        let raw = "\
ffmpeg version 8.0 Copyright (c) 2000-2025\n\
  built with Apple clang\n\
size=     768KiB time=00:02:13.16 bitrate=  47.2kbits/s speed= 264x\r\
size=    1536KiB time=00:04:27.87 bitrate=  47.0kbits/s speed= 265x\r\
[in#0 @ 0x1] Error opening input: No such file or directory\n\
Error opening input file /tmp/.vd-preprocess-3-trim-silence.tmp.mp3.\n\
Error opening input files: No such file or directory\n";
        let out = compact_tool_output(raw);
        assert!(out.contains("No such file or directory"));
        assert!(out.contains("trim-silence"));
        assert!(!out.contains("ffmpeg version"));
        assert!(!out.contains("size="));
        assert!(out.len() < 800);
    }
}
