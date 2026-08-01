//! Golden CTC checks vs dumped tensors / reference text.
//!
//! Until `scripts/dump_golden.py` and weights land, structural tests stay here;
//! heavy comparisons are `#[ignore]`.

use std::path::Path;

#[test]
fn golden_fixture_dirs_exist() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    assert!(root.join("audio").is_dir());
    assert!(root.join("expected").is_dir());
    assert!(root.join("golden").is_dir());
    assert!(root.join("audio/silence_0.2s.wav").is_file());
}

/// Layer / end-to-end tensor parity with Python reference dumps.
#[test]
#[ignore = "needs fixtures/golden tensors from scripts/dump_golden.py"]
fn golden_ctc_tensors() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/golden");
    assert!(
        golden.read_dir().unwrap().next().is_some(),
        "populate fixtures/golden before enabling this test"
    );
}
