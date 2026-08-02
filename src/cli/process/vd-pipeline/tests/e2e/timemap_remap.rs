//! E2E: 1× vs preprocess speed 2× — remapped segment timings must agree.
//!
//! Gate: `VD_PIPELINE_E2E_TIMEMAP=1` + `#[ignore]`.
//! Needs: `vd-gigaam` (CTC), `vd-preprocess`, ffmpeg/ffprobe, fixture audio.
//! Prefer: `cargo test --release`.
//!
//! See `docs/adr/0001-platform-refactoring-plan.md` §5–6.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::helpers::{
    child_available, ctc_model_ready, ffmpeg_available, gigaam_models_root, gigaam_supports_metal,
    path_with_pipeline_siblings,
};
use super::{bin, fixture, with_isolation};

/// Absolute tolerance on remapped segment end (seconds).
const ABS_EPS: f64 = 0.35;
/// Relative tolerance vs 1× utterance end.
const REL_EPS: f64 = 0.03;

#[test]
#[ignore = "experimental: VD_PIPELINE_E2E_TIMEMAP=1, ffmpeg, vd-preprocess, vd-gigaam CTC"]
fn preprocess_speed_2x_timemap_matches_1x_segments() {
    if std::env::var_os("VD_PIPELINE_E2E_TIMEMAP").as_deref() != Some(std::ffi::OsStr::new("1")) {
        eprintln!("skip: set VD_PIPELINE_E2E_TIMEMAP=1");
        return;
    }
    if !child_available("vd-gigaam") || !child_available("vd-preprocess") {
        eprintln!("skip: need vd-gigaam + vd-preprocess release/debug siblings");
        return;
    }
    if !ffmpeg_available() {
        eprintln!("skip: ffmpeg missing");
        return;
    }
    let models_root = gigaam_models_root();
    if !ctc_model_ready(&models_root) {
        eprintln!("skip: CTC model missing");
        return;
    }

    let stem = "text-beginner-russian-sauna";
    let audio_src = fixture(&format!("audio/{stem}.mp3"));
    assert!(audio_src.is_file(), "missing {}", audio_src.display());

    let end_1x = run_segments(&models_root, &audio_src, stem, /*speed*/ None);
    let end_2x = run_segments(&models_root, &audio_src, stem, Some(2.0));

    eprintln!("timemap compare: end_1x={end_1x:.3}  end_2x_remapped={end_2x:.3}");
    let abs = (end_1x - end_2x).abs();
    let rel = abs / end_1x.max(1e-6);
    assert!(
        abs <= ABS_EPS || rel <= REL_EPS,
        "remapped 2× segment end {end_2x:.3} diverges from 1× {end_1x:.3} (abs={abs:.3}, rel={rel:.3})"
    );
}

fn run_segments(
    models_root: &Path,
    audio_src: &Path,
    stem: &str,
    speed: Option<f64>,
) -> f64 {
    let dir = TempDir::new().unwrap();
    let audio_name = format!("{stem}.mp3");
    fs::copy(audio_src, dir.path().join(&audio_name)).unwrap();

    let device_line = if gigaam_supports_metal() {
        "      device: metal\n"
    } else {
        ""
    };

    let preprocess_block = if let Some(factor) = speed {
        format!(
            r#"  - use: preprocess
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
"#
        )
    } else {
        r#"  - use: transcribe
    id: transcript
    output: transcript.txt
"#
        .to_string()
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
{preprocess_block}    options:
      engine: gigaam
      model: v3_e2e_ctc
{device_line}      overwrite: true
      segments: true
      word_timestamps: true
"#,
            wd = dir.path().display(),
        ),
    )
    .unwrap();

    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.env("VD_GIGAAM_MODELS_DIR", models_root);
    cmd.env("PATH", path_with_pipeline_siblings());
    cmd.arg(&job)
        .arg("-q")
        .timeout(std::time::Duration::from_secs(1800))
        .assert()
        .success();

    let seg_path = dir.path().join("transcript.segments.json");
    assert!(
        seg_path.is_file(),
        "missing segments {}",
        seg_path.display()
    );

    if speed.is_some() {
        let tm = dir.path().join("prepared.timemap.json");
        assert!(tm.is_file(), "missing TimeMap {}", tm.display());
    }

    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&seg_path).unwrap()).unwrap();
    let end = v["segments"][0]["end"]
        .as_f64()
        .expect("segment end");
    eprintln!(
        "  speed={speed:?} segments_end={end:.3} words={}",
        v["words"].as_array().map(|a| a.len()).unwrap_or(0)
    );
    end
}
