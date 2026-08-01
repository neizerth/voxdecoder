//! Resample / mono unit tests.

use vd_gigaam::audio::resample::to_mono_16k;

#[test]
fn identity_at_16k() {
    let s = vec![0.1, -0.2, 0.3];
    let out = to_mono_16k(&s, 1, 16_000).unwrap();
    assert_eq!(out, s);
}

#[test]
fn downmix_stereo() {
    let interleaved = vec![1.0, 3.0, 2.0, 4.0];
    let mono = to_mono_16k(&interleaved, 2, 16_000).unwrap();
    assert!((mono[0] - 2.0).abs() < 1e-6);
    assert!((mono[1] - 3.0).abs() < 1e-6);
}

#[test]
fn resample_8k_to_16k_doubles_len() {
    let s: Vec<f32> = (0..800).map(|i| (i as f32 * 0.01).sin()).collect();
    let out = to_mono_16k(&s, 1, 8_000).unwrap();
    assert_eq!(out.len(), 1600);
}
