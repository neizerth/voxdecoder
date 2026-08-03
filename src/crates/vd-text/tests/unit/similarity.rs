//! `vd_text::similarity` — shared edit-distance/ratio implementation.

use vd_text::similarity::{edit_distance, similarity_ratio};

#[test]
fn identical_strings_have_zero_distance_and_full_similarity() {
    assert_eq!(edit_distance("hello", "hello"), 0);
    assert!((similarity_ratio("hello", "hello") - 1.0).abs() < f64::EPSILON);
}

#[test]
fn single_substitution_distance_is_one() {
    assert_eq!(edit_distance("cat", "cot"), 1);
}

#[test]
fn single_insertion_distance_is_one() {
    assert_eq!(edit_distance("cat", "cats"), 1);
}

#[test]
fn empty_strings_are_fully_similar() {
    assert!((similarity_ratio("", "") - 1.0).abs() < f64::EPSILON);
    assert_eq!(edit_distance("", ""), 0);
}

#[test]
fn completely_different_strings_have_low_similarity() {
    let ratio = similarity_ratio("abc", "xyz");
    assert!(ratio < 0.5, "expected low similarity, got {ratio}");
}

#[test]
fn cyrillic_text_is_handled_unicode_scalar_aware() {
    assert_eq!(edit_distance("кубернетес", "кубернетис"), 1);
    let ratio = similarity_ratio("кубернетес", "кубернетис");
    assert!(ratio > 0.85, "expected high similarity, got {ratio}");
}

#[test]
fn near_miss_asr_typo_scores_high_similarity() {
    let ratio = similarity_ratio("Deploy tomorrow morning.", "Deploy tomorrow mornin");
    assert!(ratio >= 0.85, "expected >= 0.85, got {ratio}");
}
