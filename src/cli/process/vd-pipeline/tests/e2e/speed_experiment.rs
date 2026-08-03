//! Experimental: ASR wall-time vs preprocess `speed` factor.
//!
//! Gate: `VD_PIPELINE_E2E_SPEED=1` + `#[ignore]`.
//! Needs: `vd-gigaam` (CTC), `vd-preprocess`, system `ffmpeg`, fixture audio.
//! Prefer: `cargo test --release`.
//!
//! Bands via `VD_PIPELINE_E2E_SPEED_BAND`:
//! - `low`  — `1.0 / 1.25 / 1.5 / 1.75 / 2.0`
//! - `high` — `1.0` baseline + `2.0 / 2.25 / 2.5 / 2.75 / 3.0 / 3.5 / 4.0`
//! - `all`  — union (default)
//!
//! Asserts sped-up runs finish faster than 1× with coverage not collapsing vs baseline.
//! High factors (>2×) use a progressive coverage-drop budget (atempo chain + ASR stress);
//! around 4× some clips show a real accuracy cliff — still floored, not held to the 1–2× bar.

use std::fs;
use std::time::Instant;

use tempfile::TempDir;

use super::helpers::{
    child_available, ctc_model_ready, ffmpeg_available, gigaam_models_root, gigaam_supports_metal,
    path_with_pipeline_siblings, report_step_ms, word_coverage,
};
use super::{bin, fixture, with_isolation};

const FACTORS_LOW: &[f64] = &[1.0, 1.25, 1.5, 1.75, 2.0];
const FACTORS_HIGH_TAIL: &[f64] = &[2.0, 2.25, 2.5, 2.75, 3.0, 3.5, 4.0];

/// Absolute floor: sped audio may degrade ASR a bit vs pristine 1× full-pipeline e2e (0.75).
const MIN_COVERAGE: f64 = 0.60;
/// Relative drop vs 1× baseline (tighter below 2×, looser above).
const MAX_COVERAGE_DROP: f64 = 0.15;
/// Wall/ASR slack vs theoretical `baseline / factor`.
const SPEEDUP_SLACK: f64 = 1.25;
const SPEEDUP_SLACK_HIGH: f64 = 1.60;

fn accuracy_budget(
    factor: f64,
) -> (
    f64,  /* min_cov */
    f64,  /* max_drop */
    bool, /* hard */
) {
    if factor <= 2.0 + 1e-9 {
        (MIN_COVERAGE, MAX_COVERAGE_DROP, true)
    } else if factor <= 2.5 + 1e-9 {
        (0.55, 0.20, true)
    } else if factor <= 3.0 + 1e-9 {
        (0.50, 0.25, true)
    } else {
        // >3×: measure & warn — some clips cliff hard (hobby @3.5≈0.56, @4≈0.35).
        (0.25, 0.75, false)
    }
}

fn speedup_slack(factor: f64) -> f64 {
    if factor > 2.0 + 1e-9 {
        SPEEDUP_SLACK_HIGH
    } else {
        SPEEDUP_SLACK
    }
}

#[test]
#[ignore = "experimental: VD_PIPELINE_E2E_SPEED=1, ffmpeg, vd-preprocess, vd-gigaam CTC, fixture audio"]
fn preprocess_speed_faster_than_1x() {
    if std::env::var_os("VD_PIPELINE_E2E_SPEED").as_deref() != Some(std::ffi::OsStr::new("1")) {
        eprintln!("skip preprocess_speed_faster_than_1x: set VD_PIPELINE_E2E_SPEED=1");
        return;
    }
    if !child_available("vd-gigaam") {
        eprintln!("skip: vd-gigaam not built");
        return;
    }
    if !child_available("vd-preprocess") {
        eprintln!("skip: vd-preprocess not built");
        return;
    }
    if !ffmpeg_available() {
        eprintln!("skip: ffmpeg not on PATH (or VD_FFMPEG)");
        return;
    }

    let models_root = gigaam_models_root();
    if !ctc_model_ready(&models_root) {
        eprintln!(
            "skip: missing {}/v3_e2e_ctc/model.safetensors",
            models_root.display()
        );
        return;
    }

    let factors = selected_factors();
    eprintln!("speed factors: {factors:?}");
    let clips = selected_clips();
    for (stem, audio_rel, expected_rel) in clips {
        run_speed_matrix(&models_root, stem, audio_rel, expected_rel, &factors);
    }
}

fn selected_factors() -> Vec<f64> {
    match std::env::var("VD_PIPELINE_E2E_SPEED_BAND").as_deref() {
        Ok("low") => FACTORS_LOW.to_vec(),
        Ok("high") => {
            let mut v = vec![1.0];
            v.extend_from_slice(FACTORS_HIGH_TAIL);
            v
        }
        Ok("all") | Err(_) => {
            let mut v = FACTORS_LOW.to_vec();
            for &f in FACTORS_HIGH_TAIL {
                if v.iter().all(|x| (x - f).abs() > 1e-9) {
                    v.push(f);
                }
            }
            v
        }
        Ok(other) => {
            eprintln!("unknown VD_PIPELINE_E2E_SPEED_BAND={other}; using all");
            let mut v = FACTORS_LOW.to_vec();
            for &f in FACTORS_HIGH_TAIL {
                if v.iter().all(|x| (x - f).abs() > 1e-9) {
                    v.push(f);
                }
            }
            v
        }
    }
}

fn selected_clips() -> Vec<(&'static str, &'static str, &'static str)> {
    let all = [
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
    ];
    match std::env::var("VD_PIPELINE_E2E_SPEED_CLIP").as_deref() {
        Ok("hobby") => vec![all[0]],
        Ok("sauna") => vec![all[1]],
        Ok("all") | Err(_) => all.to_vec(),
        Ok(other) => {
            eprintln!("unknown VD_PIPELINE_E2E_SPEED_CLIP={other}; using all");
            all.to_vec()
        }
    }
}

#[derive(Debug, Clone)]
struct SpeedRun {
    factor: f64,
    total_ms: u64,
    transcribe_ms: Option<u64>,
    coverage: f64,
}

fn run_speed_matrix(
    models_root: &std::path::Path,
    stem: &str,
    audio_rel: &str,
    expected_rel: &str,
    factors: &[f64],
) {
    let audio_src = fixture(audio_rel);
    let expected_src = fixture(expected_rel);
    assert!(audio_src.is_file(), "missing {}", audio_src.display());
    assert!(expected_src.is_file(), "missing {}", expected_src.display());
    let expected = fs::read_to_string(&expected_src).unwrap();

    eprintln!("=== speed experiment: {stem} ===");
    let mut runs = Vec::with_capacity(factors.len());
    for &factor in factors {
        let run = run_one_factor(models_root, stem, &audio_src, &expected, factor);
        eprintln!(
            "  factor={factor:.2}  total_ms={:>8}  transcribe_ms={:>8}  coverage={:.3}",
            run.total_ms,
            run.transcribe_ms
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".into()),
            run.coverage
        );
        runs.push(run);
    }

    let baseline = runs
        .iter()
        .find(|r| (r.factor - 1.0).abs() < f64::EPSILON)
        .expect("1.0 baseline");

    assert!(
        baseline.coverage >= MIN_COVERAGE,
        "{stem} @1.0: coverage {:.3} < {MIN_COVERAGE}",
        baseline.coverage
    );

    for run in &runs {
        if (run.factor - 1.0).abs() < f64::EPSILON {
            continue;
        }

        let (min_cov, max_drop, hard_acc) = accuracy_budget(run.factor);
        let slack = speedup_slack(run.factor);

        let drop = baseline.coverage - run.coverage;
        if drop > MAX_COVERAGE_DROP {
            eprintln!(
                "  note: @{:.2} coverage {:.3} drops {drop:.3} vs baseline {:.3} (budget max_drop={max_drop}, hard={hard_acc})",
                run.factor, run.coverage, baseline.coverage
            );
        }

        if hard_acc {
            assert!(
                run.coverage >= min_cov,
                "{stem} @{:.2}: coverage {:.3} < {min_cov}\n(baseline 1.0 was {:.3})",
                run.factor,
                run.coverage,
                baseline.coverage
            );
            assert!(
                run.coverage + max_drop >= baseline.coverage,
                "{stem} @{:.2}: coverage {:.3} dropped > {max_drop} vs baseline {:.3}",
                run.factor,
                run.coverage,
                baseline.coverage
            );
        } else {
            assert!(
                run.coverage >= min_cov,
                "{stem} @{:.2}: coverage {:.3} < soft floor {min_cov}",
                run.factor,
                run.coverage
            );
            if drop > max_drop {
                eprintln!(
                    "  WARN: @{:.2} coverage cliff {:.3} (drop {drop:.3}); speed still asserted",
                    run.factor, run.coverage
                );
            }
        }

        let max_allowed = (baseline.total_ms as f64) * slack / run.factor;
        assert!(
            (run.total_ms as f64) < max_allowed,
            "{stem} @{:.2}: total_ms {} not faster enough vs 1.0 baseline {} (allowed < {max_allowed:.0})",
            run.factor,
            run.total_ms,
            baseline.total_ms
        );

        if let (Some(base_asr), Some(run_asr)) = (baseline.transcribe_ms, run.transcribe_ms) {
            let asr_allowed = (base_asr as f64) * slack / run.factor;
            assert!(
                (run_asr as f64) < asr_allowed,
                "{stem} @{:.2}: transcribe_ms {run_asr} not faster enough vs baseline {base_asr} (allowed < {asr_allowed:.0})",
                run.factor
            );
        }
    }
}

fn run_one_factor(
    models_root: &std::path::Path,
    stem: &str,
    audio_src: &std::path::Path,
    expected: &str,
    factor: f64,
) -> SpeedRun {
    let dir = TempDir::new().unwrap();
    let audio_name = format!("{stem}.mp3");
    fs::copy(audio_src, dir.path().join(&audio_name)).unwrap();

    let device_line = if gigaam_supports_metal() {
        "      device: metal\n"
    } else {
        ""
    };

    let job = dir.path().join("job.yaml");
    fs::write(
        &job,
        format!(
            r#"version: 1
working_dir: {wd}
input:
  audio: {audio_name}
steps:
  - use: preprocess
    id: prepared
    output: prepared.wav
    options:
      provider: ffmpeg
      overwrite: true
      filters:
        - type: mono
        - type: resample
          rate: 16000
        - type: speed
          factor: {factor}
  - use: transcribe
    id: transcript
    input: prepared
    output: transcript.txt
    options:
      engine: gigaam
      model: v3_e2e_ctc
{device_line}      overwrite: true
  - use: fix-casing
    input: transcript
    options:
      overwrite: true
  - use: fix-asr
    options:
      overwrite: true
  - use: fix-terms
    options:
      overwrite: true
"#,
            wd = dir.path().display(),
        ),
    )
    .unwrap();

    let cfg = dir.path().join("config.toml");
    let report_path = dir.path().join("report.json");
    let wall = Instant::now();

    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.env("VD_GIGAAM_MODELS_DIR", models_root);
    cmd.env("PATH", path_with_pipeline_siblings());
    cmd.arg(&job)
        .arg("-q")
        .arg("--report")
        .arg(&report_path)
        .timeout(std::time::Duration::from_secs(1800))
        .assert()
        .success();

    let wall_ms = wall.elapsed().as_millis() as u64;

    let out = dir.path().join("transcript.fixed.txt");
    assert!(out.is_file(), "missing transcript {}", out.display());
    let got = fs::read_to_string(&out).unwrap();
    let coverage = word_coverage(expected, &got);

    let (total_ms, transcribe_ms) = if report_path.is_file() {
        let report: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&report_path).unwrap()).unwrap();
        let total = report["duration_ms"].as_u64().unwrap_or(wall_ms);
        let asr = report_step_ms(&report, "transcribe");
        (total, asr)
    } else {
        (wall_ms, None)
    };

    SpeedRun {
        factor,
        total_ms,
        transcribe_ms,
        coverage,
    }
}
