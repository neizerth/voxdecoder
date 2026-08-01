//! Light e2e runs (real child CLIs when available).

use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;

use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

use super::{bin, fixture, with_isolation};

fn child_available(name: &str) -> bool {
    if let Ok(p) = which_near_pipeline(name) {
        return p.exists();
    }
    StdCommand::new(name)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn which_near_pipeline(name: &str) -> Result<PathBuf, ()> {
    let pipe = cargo_bin!("vd-pipeline");
    let candidate = pipe.parent().ok_or(())?.join(name);
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(())
    }
}

#[test]
fn run_fix_only() {
    if !(child_available("vd-fix-casing")
        && child_available("vd-fix-asr")
        && child_available("vd-fix-terms"))
    {
        eprintln!("skip run_fix_only: fix CLIs not built");
    }

    let dir = TempDir::new().unwrap();
    let sample = dir.path().join("sample.txt");
    fs::copy(fixture("sample.txt"), &sample).unwrap();
    let job = dir.path().join("job.yaml");
    fs::write(
        &job,
        format!(
            "version: 1\nworking_dir: {}\nsteps:\n  - use: fix-casing\n    input: sample.txt\n    options:\n      overwrite: true\n  - use: fix-asr\n    options:\n      overwrite: true\n  - use: fix-terms\n    options:\n      overwrite: true\n",
            dir.path().display()
        ),
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(job).arg("-q").assert().success();

    let fixed = dir.path().join("sample.fixed.txt");
    assert!(
        fixed.exists(),
        "expected {} after fix chain",
        fixed.display()
    );
}

#[test]
fn run_prepare_context() {
    if !child_available("vd-assets") {
        eprintln!("skip run_prepare_context: vd-assets not built");
    }

    let dir = TempDir::new().unwrap();
    let docs = dir.path().join("docs");
    fs::create_dir(&docs).unwrap();
    fs::copy(fixture("docs/notes.md"), docs.join("notes.md")).unwrap();
    let assets = dir.path().join("assets");
    let job = dir.path().join("job.yaml");
    fs::write(
        &job,
        format!(
            "version: 1\nworking_dir: {}\ncontext:\n  docs: docs\n  assets: assets\nsteps:\n  - use: prepare-context\n",
            dir.path().display()
        ),
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(job).arg("-q").assert().success();

    assert!(assets.join("terms.yml").exists() || assets.join("md").exists());
}

#[test]
fn run_diarize_stub() {
    if !child_available("vd-diarize") {
        eprintln!("skip run_diarize_stub: vd-diarize not built");
        return;
    }

    let dir = TempDir::new().unwrap();
    let audio = dir.path().join("meeting.wav");
    fs::write(&audio, vec![0u8; 16_000]).unwrap();
    let job = dir.path().join("job.yaml");
    fs::write(
        &job,
        format!(
            "version: 1\nworking_dir: {}\ninput:\n  audio: meeting.wav\nsteps:\n  - use: diarize\n    options:\n      backend:\n        provider: stub\n      overwrite: true\n",
            dir.path().display()
        ),
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    // Prefer sibling binary from cargo target dir.
    if let Ok(d) = which_near_pipeline("vd-diarize") {
        if let Some(parent) = d.parent() {
            let path = env_path_prepend(parent);
            cmd.env("PATH", path);
        }
    }
    cmd.arg(&job).arg("-q").assert().success();

    let out = dir.path().join("meeting.diarization.json");
    assert!(out.exists(), "expected {}", out.display());
}

#[test]
#[ignore = "requires VD_PIPELINE_E2E_FULL=1, audio, and vd-gigaam"]
fn run_full_pipeline() {
    // Placeholder for gated full ASR e2e when VD_PIPELINE_E2E_FULL=1.
}

fn env_path_prepend(dir: &std::path::Path) -> std::ffi::OsString {
    use std::env;
    let mut out = dir.as_os_str().to_owned();
    if let Some(rest) = env::var_os("PATH") {
        out.push(":");
        out.push(rest);
    }
    out
}
