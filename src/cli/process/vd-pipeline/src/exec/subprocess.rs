//! Spawn child CLIs for capabilities.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::job::{ArgValue, Capability};

use super::{Binder, ExecError, InvokeRequest, InvokeResult};

#[derive(Debug, Default)]
pub struct SubprocessBinder;

impl Binder for SubprocessBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        match req.capability {
            Capability::Transcribe => run_transcribe(req),
            Capability::PrepareContext => run_prepare_context(req),
            Capability::FixCasing => run_fix(req, "vd-fix-casing"),
            Capability::FixAsr => run_fix(req, "vd-fix-asr"),
            Capability::FixTerms => run_fix(req, "vd-fix-terms"),
        }
    }
}

fn run_transcribe(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let engine = req
        .options
        .get("engine")
        .and_then(ArgValue::as_string)
        .unwrap_or_else(|| "gigaam".into());
    if engine == "whisper" {
        return Err(ExecError::Reserved(
            "whisper is reserved; vd-whisper is not available yet".into(),
        ));
    }
    if engine != "gigaam" {
        return Err(ExecError::Step(format!(
            "unknown transcribe engine: {engine}"
        )));
    }

    let bin = find_bin("vd-gigaam")?;
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-q".into(),
    ];
    if let Some(m) = req.options.get("model").and_then(ArgValue::as_string) {
        args.push("-m".into());
        args.push(m);
    }
    if let Some(d) = req.options.get("device").and_then(ArgValue::as_string) {
        args.push("--device".into());
        args.push(d);
    }
    if req.options.get("flash").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--flash".into());
    }
    if req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--overwrite".into());
    }
    if let Some(o) = &req.output {
        args.push("-o".into());
        args.push(o.display().to_string());
    } else if let Some(d) = &req.output_dir {
        args.push("-d".into());
        args.push(d.display().to_string());
    }

    run_cmd(&bin, &args, &req.working_dir)?;
    let out = infer_gigaam_output(req);
    Ok(InvokeResult {
        primary_output: out,
    })
}

fn infer_gigaam_output(req: &InvokeRequest) -> PathBuf {
    if let Some(o) = &req.output {
        return o.clone();
    }
    let stem = req
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    if let Some(d) = &req.output_dir {
        return d.join(format!("{stem}.txt"));
    }
    req.input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.txt"))
}

fn run_prepare_context(req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
    let bin = find_bin("vd-assets")?;
    let out_dir = req
        .output
        .clone()
        .or_else(|| req.context_assets.clone())
        .unwrap_or_else(|| req.working_dir.join(".voxdecoder"));
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-o".into(),
        out_dir.display().to_string(),
        "-q".into(),
    ];
    if req.options.get("ocr").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--ocr".into());
    }
    if req.options.get("force").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--force".into());
    }
    run_cmd(&bin, &args, &req.working_dir)?;
    Ok(InvokeResult {
        primary_output: out_dir,
    })
}

fn run_fix(req: &InvokeRequest, bin_name: &str) -> Result<InvokeResult, ExecError> {
    let bin = find_bin(bin_name)?;
    let mut args = vec![
        "run".into(),
        "-i".into(),
        req.input.display().to_string(),
        "-q".into(),
    ];
    if let Some(o) = &req.output {
        args.push("-o".into());
        args.push(o.display().to_string());
    } else if let Some(d) = &req.output_dir {
        args.push("-d".into());
        args.push(d.display().to_string());
    }
    if req.options.get("overwrite").and_then(ArgValue::as_bool) == Some(true) {
        args.push("--overwrite".into());
    }
    if let Some(lang) = req.options.get("language").and_then(ArgValue::as_string) {
        args.push("-l".into());
        args.push(lang);
    }
    if bin_name == "vd-fix-asr" {
        if let Some(ctx) = &req.context_assets {
            args.push("--context".into());
            args.push(ctx.display().to_string());
        }
    }
    if bin_name == "vd-fix-terms" {
        if let Some(ctx) = &req.context_assets {
            args.push("--terms".into());
            args.push(ctx.display().to_string());
        }
    }
    run_cmd(&bin, &args, &req.working_dir)?;
    Ok(InvokeResult {
        primary_output: infer_fix_output(req),
    })
}

fn infer_fix_output(req: &InvokeRequest) -> PathBuf {
    if let Some(o) = &req.output {
        return o.clone();
    }
    let stem = req
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let ext = req
        .input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt");
    // strip prior .fixed
    let stem = stem.strip_suffix(".fixed").unwrap_or(stem);
    if let Some(d) = &req.output_dir {
        return d.join(format!("{stem}.fixed.{ext}"));
    }
    req.input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}.fixed.{ext}"))
}

fn run_cmd(bin: &Path, args: &[String], cwd: &Path) -> Result<(), ExecError> {
    let status = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| ExecError::Step(format!("{}: {e}", bin.display())))?;
    if status.success() {
        Ok(())
    } else {
        Err(ExecError::Step(format!(
            "{} exited {}",
            bin.display(),
            status.code().unwrap_or(1)
        )))
    }
}

fn find_bin(name: &str) -> Result<PathBuf, ExecError> {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
            #[cfg(windows)]
            {
                let bat = dir.join(format!("{name}.exe"));
                if bat.is_file() {
                    return Ok(bat);
                }
            }
        }
    }
    Ok(PathBuf::from(name))
}
