//! Audio → mel integration against fixtures.

use std::path::PathBuf;

use vd_giga::audio;
use vd_giga::gigaam::model::GigaModel;

fn fixture_wav() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/audio/silence_0.2s.wav")
}

#[test]
fn fixture_wav_to_log_mel() {
    let pcm = audio::load_pcm16k_mono(&fixture_wav()).unwrap();
    assert_eq!(pcm.len(), 3200);
    let mel = GigaModel::extract_features(&pcm);
    assert_eq!(mel.n_mels, 64);
    assert_eq!(mel.n_frames, 19);
    assert!(mel.data.iter().all(|v| v.is_finite()));
}
