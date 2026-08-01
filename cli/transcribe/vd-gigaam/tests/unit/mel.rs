//! Mel frontend unit tests.

use vd_gigaam::gigaam::frontend::mel::{
    extract_log_mel, hz_to_mel_htk, mel_to_hz_htk, MelConfig,
};

#[test]
fn gigaam_frame_count_no_center() {
    let cfg = MelConfig::gigaam();
    // 0.2s @ 16k = 3200 samples → (3200-320)/160 + 1 = 19
    assert_eq!(cfg.n_frames(3200), 19);
    assert_eq!(cfg.n_frames(319), 0);
    assert_eq!(cfg.n_frames(320), 1);
}

#[test]
fn htk_roundtrip() {
    let hz = 1000.0;
    let mel = hz_to_mel_htk(hz);
    assert!((mel_to_hz_htk(mel) - hz).abs() < 1e-6);
}

#[test]
fn log_mel_shape_and_finite() {
    let cfg = MelConfig::gigaam();
    let mut samples = vec![0.0f32; 3200];
    // mild tone so energy > floor
    for (i, s) in samples.iter_mut().enumerate() {
        *s = (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16_000.0).sin() * 0.2;
    }
    let mel = extract_log_mel(&samples, cfg);
    assert_eq!(mel.n_mels, 64);
    assert_eq!(mel.n_frames, 19);
    assert_eq!(mel.data.len(), 64 * 19);
    assert!(mel.data.iter().all(|v| v.is_finite()));
    // SpecScaler floor around ln(1e-9) ≈ -20.7
    assert!(mel.data.iter().all(|&v| v >= -21.0));
}

#[test]
fn silence_near_log_floor() {
    let cfg = MelConfig::gigaam();
    let samples = vec![0.0f32; 1600];
    let mel = extract_log_mel(&samples, cfg);
    let mean = mel.data.iter().sum::<f32>() / mel.data.len() as f32;
    assert!(
        (mean - (1e-9f32).ln()).abs() < 0.5,
        "silence should sit near ln(1e-9), got {mean}"
    );
}
