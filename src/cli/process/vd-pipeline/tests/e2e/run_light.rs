//! Light e2e runs (real child CLIs when available).

use std::fs;

use tempfile::TempDir;

use super::helpers::{
    child_available, ctc_model_ready, env_path_prepend, gigaam_models_root, gigaam_supports_metal,
    which_near_pipeline, word_coverage,
};
use super::{bin, fixture, with_isolation};

#[test]
fn run_fix_only() {
    if !(child_available("vd-fix-casing")
        && child_available("vd-fix-asr")
        && child_available("vd-fix-terms")
        && child_available("vd-fix-layout"))
    {
        eprintln!("skip run_fix_only: fix CLIs not built");
        return;
    }

    let dir = TempDir::new().unwrap();
    let sample = dir.path().join("sample.txt");
    fs::copy(fixture("sample.txt"), &sample).unwrap();
    let job = dir.path().join("job.yaml");
    fs::write(
        &job,
        format!(
            "version: 1\nworking_dir: {}\nsteps:\n  - use: fix-casing\n    input: sample.txt\n    options:\n      overwrite: true\n  - use: fix-asr\n    options:\n      overwrite: true\n  - use: fix-terms\n    options:\n      overwrite: true\n  - use: fix-layout\n    options:\n      overwrite: true\n",
            dir.path().display()
        ),
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.arg(job).arg("-q").assert().success();

    let fixed = dir.path().join(".voxdecoder/work/sample.fixed.txt");
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
    if let Ok(d) = which_near_pipeline("vd-diarize") {
        if let Some(parent) = d.parent() {
            let path = env_path_prepend(parent);
            cmd.env("PATH", path);
        }
    }
    cmd.arg(&job).arg("-q").assert().success();

    let out = dir.path().join(".voxdecoder/work/meeting.diarization.json");
    assert!(out.exists(), "expected {}", out.display());
}

#[test]
#[ignore = "requires VD_PIPELINE_E2E_FULL=1, audio, converted CTC weights, and vd-gigaam"]
fn run_full_pipeline() {
    if std::env::var_os("VD_PIPELINE_E2E_FULL").as_deref() != Some(std::ffi::OsStr::new("1")) {
        eprintln!("skip run_full_pipeline: set VD_PIPELINE_E2E_FULL=1");
        return;
    }
    if !child_available("vd-gigaam") {
        eprintln!("skip run_full_pipeline: vd-gigaam not built");
        return;
    }

    let models_root = gigaam_models_root();
    if !ctc_model_ready(&models_root) {
        eprintln!(
            "skip run_full_pipeline: missing {}/v3_e2e_ctc/model.safetensors",
            models_root.display()
        );
        return;
    }

    for (stem, audio_rel, expected_rel) in [
        (
            "02-03-strannoye-hobby",
            "audio/02-03-strannoye-hobby.mp3",
            "audio/02-03-strannoye-hobby.expected.txt",
        ),
        (
            "text-beginner-russian-sauna",
            "audio/text-beginner-russian-sauna.mp3",
            "audio/text-beginner-russian-sauna.expected.txt",
        ),
    ] {
        run_full_pipeline_clip(&models_root, stem, audio_rel, expected_rel);
    }
}

fn run_full_pipeline_clip(
    models_root: &std::path::Path,
    stem: &str,
    audio_rel: &str,
    expected_rel: &str,
) {
    let audio_src = fixture(audio_rel);
    let expected_src = fixture(expected_rel);
    assert!(
        audio_src.is_file(),
        "missing audio fixture {}",
        audio_src.display()
    );
    assert!(
        expected_src.is_file(),
        "missing expected transcript {}",
        expected_src.display()
    );

    let dir = TempDir::new().unwrap();
    let audio_name = format!("{stem}.mp3");
    let audio = dir.path().join(&audio_name);
    fs::copy(&audio_src, &audio).unwrap();
    let job = dir.path().join("job.yaml");
    let device_line = if gigaam_supports_metal() {
        eprintln!("=== {stem}: device=metal ===");
        "      device: metal\n"
    } else {
        eprintln!("=== {stem}: device=default (no metal in vd-gigaam) ===");
        ""
    };
    fs::write(
        &job,
        format!(
            "version: 1\nworking_dir: {}\ninput:\n  audio: {audio_name}\nsteps:\n  - use: transcribe\n    options:\n      engine: gigaam\n      model: v3_e2e_ctc\n{device_line}      overwrite: true\n  - use: fix-casing\n    options:\n      overwrite: true\n  - use: fix-asr\n    options:\n      overwrite: true\n  - use: fix-terms\n    options:\n      overwrite: true\n  - use: fix-layout\n    options:\n      overwrite: true\n",
            dir.path().display()
        ),
    )
    .unwrap();
    let cfg = dir.path().join("config.toml");

    let report_path = dir.path().join("report.json");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.env("VD_GIGAAM_MODELS_DIR", models_root);
    if let Ok(d) = which_near_pipeline("vd-gigaam") {
        if let Some(parent) = d.parent() {
            cmd.env("PATH", env_path_prepend(parent));
        }
    }
    cmd.arg(&job)
        .arg("-q")
        .arg("--report")
        .arg(&report_path)
        .timeout(std::time::Duration::from_secs(1800))
        .assert()
        .success();

    let out = dir
        .path()
        .join(".voxdecoder/work")
        .join(format!("{stem}.fixed.txt"));
    assert!(
        out.is_file(),
        "expected cleaned transcript {}",
        out.display()
    );
    assert!(
        report_path.is_file(),
        "expected execution report {}",
        report_path.display()
    );

    let report_raw = fs::read_to_string(&report_path).unwrap();
    let report: serde_json::Value = serde_json::from_str(&report_raw).unwrap();
    eprintln!(
        "=== report {stem} total_ms={} status={} ===",
        report["duration_ms"], report["status"]
    );
    if let Some(steps) = report["steps"].as_array() {
        for s in steps {
            eprintln!(
                "  {:>16}  {:>8} ms  {}",
                s["capability"].as_str().unwrap_or("?"),
                s["duration_ms"],
                s["status"].as_str().unwrap_or("?")
            );
        }
    }

    let got = fs::read_to_string(&out).unwrap();
    let expected = fs::read_to_string(&expected_src).unwrap();
    let coverage = word_coverage(&expected, &got);
    assert!(
        coverage >= 0.75,
        "{stem}: transcript word coverage {coverage:.2} < 0.75\n--- expected ---\n{expected}\n--- got ---\n{got}"
    );
}
