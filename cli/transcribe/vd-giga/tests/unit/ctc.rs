//! CTC greedy collapse unit tests.

use vd_giga::gigaam::decoder::ctc::{greedy_collapse, greedy_collapse_with_frames};

#[test]
fn collapses_blank_and_repeats() {
    // blank=0: _ a a _ b b b _ a → a b a
    let path = [0, 1, 1, 0, 2, 2, 2, 0, 1];
    assert_eq!(greedy_collapse(&path, 0), vec![1, 2, 1]);
}

#[test]
fn keeps_frames_of_emissions() {
    let path = [0, 1, 1, 0, 2];
    let (tok, fr) = greedy_collapse_with_frames(&path, 0);
    assert_eq!(tok, vec![1, 2]);
    assert_eq!(fr, vec![1, 4]);
}

#[test]
fn empty_all_blank() {
    assert!(greedy_collapse(&[0, 0, 0], 0).is_empty());
}
