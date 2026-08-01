//! HTTP download + optional `convert_ckpt.py` for install.

use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use md5::{Digest, Md5};

use crate::gigaam::catalog::{self, resolve_model_name};
use super::{ModelPaths, WeightsError};

pub type ProgressFn<'a> = dyn FnMut(u64, Option<u64>) + 'a;

#[derive(Debug)]
pub enum InstallOutcome {
    /// Converted SafeTensors already present under download_root.
    AlreadyPresent(PathBuf),
    /// Freshly converted (ckpt may have been reused or downloaded).
    Installed(PathBuf),
}

/// Download catalog weights (and tokenizer if needed), then try SafeTensors convert.
pub fn install_model(
    download_root: &Path,
    model: &str,
    force: bool,
    mut on_progress: Option<&mut ProgressFn<'_>>,
) -> Result<InstallOutcome, WeightsError> {
    let name = resolve_model_name(model).to_string();
    if !catalog::is_catalog_name(&name) {
        return Err(WeightsError::DownloadNotImplemented(model.into()));
    }
    fs::create_dir_all(download_root)?;
    scrub_partial_downloads(download_root, &name);

    if !force {
        if let Ok(paths) = super::resolve_converted(download_root, &name) {
            return Ok(InstallOutcome::AlreadyPresent(paths.safetensors));
        }
    } else {
        // Drop converted tree so we reconvert (and re-fetch ckpt if missing).
        let out_dir = download_root.join(&name);
        if out_dir.is_dir() {
            let _ = fs::remove_dir_all(&out_dir);
        }
        for flat in [
            download_root.join(format!("{name}.safetensors")),
            download_root.join(format!("{name}.json")),
        ] {
            let _ = fs::remove_file(flat);
        }
    }

    let managed_ckpt = download_root.join(format!("{name}.ckpt"));
    let ckpt_path = if managed_ckpt.is_file() {
        managed_ckpt.clone()
    } else if let Some(ext) = super::find_external_ckpt(&name) {
        // Optional: reuse Python GigaAM cache without a second CDN download.
        ext
    } else {
        let url = catalog::ckpt_url(&name);
        download_file(&url, &managed_ckpt, &mut on_progress)?;
        if let Some(expected) = catalog::ckpt_md5(&name) {
            let got = file_md5(&managed_ckpt)?;
            if got != expected {
                let _ = fs::remove_file(&managed_ckpt);
                return Err(WeightsError::Checksum {
                    path: managed_ckpt,
                    expected: expected.into(),
                    got,
                });
            }
        }
        managed_ckpt.clone()
    };

    if catalog::needs_tokenizer(&name) {
        let tok = download_root.join(format!("{name}_tokenizer.model"));
        if !tok.is_file() {
            let url = catalog::tokenizer_url(&name);
            download_file(&url, &tok, &mut on_progress)?;
        }
    }

    let out_dir = download_root.join(&name);
    match try_convert(&ckpt_path, &out_dir) {
        Ok(paths) => Ok(InstallOutcome::Installed(paths.safetensors)),
        Err(conv_err) => Err(WeightsError::Convert(format!(
            "have checkpoint {} but convert failed: {conv_err}. \
             Run: python cli/transcribe/vd-gigaam/scripts/convert_ckpt.py {} -o {}",
            ckpt_path.display(),
            ckpt_path.display(),
            out_dir.display()
        ))),
    }
}

/// Remove interrupted `{name}.tmp` leftovers under download_root.
pub fn scrub_partial_downloads(download_root: &Path, name: &str) {
    let tmp = download_root.join(format!("{name}.tmp"));
    if tmp.is_file() {
        let _ = fs::remove_file(&tmp);
    }
    // ureq path uses `dest.with_extension("tmp")` → `v2_rnnt.tmp` for `v2_rnnt.ckpt`
    let ckpt_tmp = download_root.join(format!("{name}.ckpt.tmp"));
    if ckpt_tmp.is_file() {
        let _ = fs::remove_file(&ckpt_tmp);
    }
}

fn download_file(
    url: &str,
    dest: &Path,
    on_progress: &mut Option<&mut ProgressFn<'_>>,
) -> Result<(), WeightsError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = dest.with_extension("tmp");
    let _ = fs::remove_file(&tmp);
    let result = (|| {
        let response = ureq::get(url)
            .call()
            .map_err(|e| WeightsError::Download(format!("{url}: {e}")))?;
        let total = response
            .header("Content-Length")
            .and_then(|s| s.parse::<u64>().ok());
        let mut reader = response.into_reader();
        let mut file = File::create(&tmp)?;
        let mut buf = [0u8; 64 * 1024];
        let mut done = 0u64;
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| WeightsError::Download(e.to_string()))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            done += n as u64;
            if let Some(cb) = on_progress.as_mut() {
                cb(done, total);
            }
        }
        file.flush()?;
        drop(file);
        fs::rename(&tmp, dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn file_md5(path: &Path) -> Result<String, WeightsError> {
    let mut file = File::open(path)?;
    let mut hasher = Md5::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn try_convert(ckpt: &Path, out_dir: &Path) -> Result<ModelPaths, WeightsError> {
    let script = find_convert_script().ok_or_else(|| {
        WeightsError::Convert(
            "convert_ckpt.py not found (set VD_GIGAAM_CONVERT_SCRIPT or use repo scripts/)".into(),
        )
    })?;
    let python = find_python(&script);
    fs::create_dir_all(out_dir)?;
    let output = Command::new(&python)
        .arg(&script)
        .arg(ckpt)
        .arg("-o")
        .arg(out_dir)
        .output()
        .map_err(|e| WeightsError::Convert(format!("spawn {python}: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(WeightsError::Convert(err.trim().to_string()));
    }
    let safetensors = out_dir.join("model.safetensors");
    let card = out_dir.join("model.json");
    if !safetensors.is_file() || !card.is_file() {
        return Err(WeightsError::Convert(
            "convert finished but model.safetensors/model.json missing".into(),
        ));
    }
    Ok(ModelPaths {
        dir: out_dir.to_path_buf(),
        safetensors,
        card,
    })
}

fn find_convert_script() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VD_GIGAAM_CONVERT_SCRIPT") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/convert_ckpt.py");
    if manifest.is_file() {
        return Some(manifest);
    }
    None
}

fn find_python(script: &Path) -> String {
    if let Ok(p) = std::env::var("VD_GIGAAM_PYTHON") {
        return p;
    }
    if let Some(dir) = script.parent() {
        let venv = dir.join(".venv/bin/python");
        if venv.is_file() {
            return venv.to_string_lossy().into_owned();
        }
        let venv_win = dir.join(".venv/Scripts/python.exe");
        if venv_win.is_file() {
            return venv_win.to_string_lossy().into_owned();
        }
    }
    "python3".into()
}
