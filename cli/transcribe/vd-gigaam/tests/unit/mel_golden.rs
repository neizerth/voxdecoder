//! Compare Rust HTK log-mel vs torchaudio dump from `scripts/dump_golden.py`.

use std::path::PathBuf;

use serde::Deserialize;
use vd_gigaam::audio;
use vd_gigaam::gigaam::frontend::mel::{extract_log_mel, MelConfig};

#[derive(Deserialize)]
struct GoldenMel {
    shape: [usize; 2],
    layout: String,
    data: Vec<f32>,
}

#[test]
fn rust_log_mel_matches_torchaudio_golden() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let golden_path = root.join("fixtures/expected/silence_0.2s_logmel.json");
    let wav = root.join("fixtures/audio/silence_0.2s.wav");
    assert!(golden_path.is_file(), "missing {golden_path:?} — run dump_golden.py");

    let golden: GoldenMel = serde_json::from_str(&std::fs::read_to_string(&golden_path).unwrap())
        .expect("golden json");
    assert_eq!(golden.layout, "n_mels,time");
    let [n_mels, n_frames] = golden.shape;

    let pcm = audio::load_pcm16k_mono(&wav).unwrap();
    let mel = extract_log_mel(&pcm, MelConfig::gigaam());
    assert_eq!(mel.n_mels, n_mels);
    assert_eq!(mel.n_frames, n_frames);

    // Golden is row-major [n_mels, time]; Rust mel stores [time, n_mels].
    let rust = mel.as_n_mels_by_time();
    assert_eq!(rust.len(), golden.data.len());

    let mut max_abs = 0.0f32;
    let mut sum_abs = 0.0f32;
    for (a, b) in rust.iter().zip(golden.data.iter()) {
        let d = (a - b).abs();
        max_abs = max_abs.max(d);
        sum_abs += d;
    }
    let mae = sum_abs / rust.len() as f32;
    // Silence sits on ln(1e-9); allow small STFT / filterbank differences.
    assert!(
        max_abs < 0.15,
        "max abs diff {max_abs} (mae {mae}) exceeds tolerance"
    );
    assert!(mae < 0.05, "mae {mae} too high");
}
