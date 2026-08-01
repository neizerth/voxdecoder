//! Audio load unit tests against fixtures.

use std::path::PathBuf;

use vd_giga::audio::load_pcm16k_mono;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/audio/silence_0.2s.wav")
}

#[test]
fn loads_silence_fixture_as_16k_mono() {
    let pcm = load_pcm16k_mono(&fixture()).unwrap();
    assert_eq!(pcm.len(), 3200);
    assert!(pcm.iter().all(|s| s.abs() < 1e-3));
}
