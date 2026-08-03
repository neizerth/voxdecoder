//! Detection signal logic (ADR 0012 §2): exact dup, near dup, non-overlap
//! exclusion, same-speaker exclusion. Pure function — no I/O involved.

use vd_fix_overlap::overlap::{
    detect_duplicates, DetectOptions, DuplicateKind, TrimAction, Utterance,
};

fn utt(speaker: &str, text: &str, start_ms: u64, end_ms: u64) -> Utterance {
    Utterance {
        speaker: speaker.to_string(),
        text: text.to_string(),
        start_ms,
        end_ms,
    }
}

#[test]
fn default_options_are_conservative() {
    let d = DetectOptions::default();
    assert!(d.similarity_threshold > 0.5 && d.similarity_threshold <= 1.0);
    assert!(d.max_gap_ms > 0);
}

#[test]
fn exact_duplicate_across_speakers_is_flagged() {
    let utterances = vec![
        utt("A", "Let's deploy tomorrow.", 1000, 3000),
        utt("B", "let's deploy tomorrow", 1200, 3200),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs.len(), 1);
    let p = &pairs[0];
    assert_eq!(p.kind, DuplicateKind::Exact);
    assert!((p.similarity - 1.0).abs() < f64::EPSILON);
    // Earlier start survives.
    assert_eq!(p.keep, 0);
    assert_eq!(p.drop, 1);
}

#[test]
fn near_duplicate_via_edit_distance_is_flagged() {
    let utterances = vec![
        utt("A", "Deploy tomorrow morning.", 0, 2000),
        // Missing a trailing letter — near-identical, not exact.
        utt("B", "Deploy tomorrow mornin", 100, 2100),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs.len(), 1);
    let p = &pairs[0];
    assert_eq!(p.kind, DuplicateKind::Near);
    assert!(p.similarity < 1.0 && p.similarity >= 0.85);
}

#[test]
fn below_similarity_threshold_is_not_flagged() {
    let utterances = vec![
        utt("A", "Deploy tomorrow morning.", 0, 2000),
        utt("B", "Completely different remark entirely.", 100, 2100),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert!(pairs.is_empty());
}

#[test]
fn non_overlapping_and_far_apart_pairs_are_not_flagged() {
    let utterances = vec![
        utt("A", "Let's deploy tomorrow.", 0, 1000),
        // Same text, but far outside the default max_gap_ms (500ms).
        utt("B", "Let's deploy tomorrow.", 5000, 6000),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert!(pairs.is_empty());
}

#[test]
fn close_but_not_overlapping_pairs_are_flagged() {
    let utterances = vec![
        utt("A", "Let's deploy tomorrow.", 0, 1000),
        // 200ms gap after A ends — within default max_gap_ms (500ms), no overlap.
        utt("B", "Let's deploy tomorrow.", 1200, 2200),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs.len(), 1);
}

#[test]
fn same_speaker_repeats_are_never_flagged() {
    let utterances = vec![
        utt("A", "Let's deploy tomorrow.", 0, 1000),
        utt("A", "Let's deploy tomorrow.", 1000, 2000),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert!(pairs.is_empty());
}

#[test]
fn keep_is_always_the_earlier_starting_utterance_regardless_of_array_order() {
    // Index 0 in the array starts *later* than index 1.
    let utterances = vec![
        utt("A", "Let's deploy tomorrow.", 5000, 6000),
        utt("B", "Let's deploy tomorrow.", 4800, 5800),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].keep, 1);
    assert_eq!(pairs[0].drop, 0);
}

#[test]
fn empty_text_is_never_flagged() {
    let utterances = vec![utt("A", "", 0, 1000), utt("B", "", 100, 1100)];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert!(pairs.is_empty());
}

#[test]
fn custom_thresholds_are_respected() {
    let utterances = vec![
        utt("A", "Deploy tomorrow morning.", 0, 2000),
        utt("B", "Deploy tomorrow mornin", 100, 2100),
    ];
    let strict = DetectOptions {
        similarity_threshold: 0.99,
        max_gap_ms: 500,
    };
    assert!(detect_duplicates(&utterances, &strict).is_empty());

    let lenient = DetectOptions {
        similarity_threshold: 0.5,
        max_gap_ms: 500,
    };
    assert_eq!(detect_duplicates(&utterances, &lenient).len(), 1);
}

#[test]
fn exact_duplicate_trim_is_remove_whole() {
    let utterances = vec![
        utt("A", "Let's deploy tomorrow.", 1000, 3000),
        utt("B", "let's deploy tomorrow", 1200, 3200),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs[0].trim, TrimAction::RemoveWhole);
}

#[test]
fn drop_containing_keep_plus_unique_tail_is_trimmed_not_removed() {
    // B (drop, later start) repeats A's whole sentence then adds a short
    // remainder A never said — deleting B outright would lose "ok".
    // Similarity is edit-distance-over-full-text, so the added remainder
    // has to stay small relative to the shared text to still clear the
    // default 0.85 threshold and get detected as a pair at all.
    let utterances = vec![
        utt("A", "Deploy tomorrow morning", 0, 1000),
        utt("B", "Deploy tomorrow morning ok", 100, 1100),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].keep, 0);
    assert_eq!(pairs[0].drop, 1);
    assert_eq!(pairs[0].trim, TrimAction::TrimTo("ok".to_string()));
}

#[test]
fn drop_containing_keep_as_suffix_trims_the_leading_remainder() {
    let utterances = vec![
        utt("A", "tomorrow morning update", 0, 1000),
        utt("B", "ok tomorrow morning update", 100, 1100),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].trim, TrimAction::TrimTo("ok".to_string()));
}

#[test]
fn three_way_only_flags_qualifying_pairs() {
    let utterances = vec![
        utt("A", "Let's deploy tomorrow.", 0, 1000),
        utt("B", "Let's deploy tomorrow.", 100, 1100),
        utt("C", "Totally unrelated statement.", 50, 1050),
    ];
    let pairs = detect_duplicates(&utterances, &DetectOptions::default());
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].keep, 0);
    assert_eq!(pairs[0].drop, 1);
}
