//! Disfluency cleanup: mode gating, filler removal, false starts, protected phrases.

use vd_fix_disfluency::disfluency::{DisfluencyFixer, DisfluencyLoadOptions};
use vd_fix_disfluency::types::{FixOptions, Language, Mode};

fn fixer(language: Language, mode: Mode) -> DisfluencyFixer {
    DisfluencyFixer::load(DisfluencyLoadOptions { language, mode }).unwrap()
}

fn fix(text: &str, language: Language, mode: Mode) -> String {
    fixer(language, mode)
        .fix_text(text, FixOptions::default())
        .unwrap()
        .text
}

#[test]
fn mode_off_never_changes_text() {
    let input = "Привет, эээ, как дела?";
    let result = fixer(Language::Ru, Mode::Off)
        .fix_text(input, FixOptions::default())
        .unwrap();
    assert!(!result.changed);
    assert_eq!(result.text, input);
}

#[test]
fn light_removes_isolated_filler() {
    let out = fix("Привет, эээ, как дела?", Language::Ru, Mode::Light);
    assert_eq!(out, "Привет, как дела?");
}

#[test]
fn light_removes_isolated_english_filler() {
    let out = fix("Well, um, I think so.", Language::En, Mode::Light);
    assert_eq!(out, "Well, I think so.");
}

#[test]
fn filler_tables_are_per_language() {
    // "um" is not in the ru filler table, so ru mode must leave it alone.
    let out = fix("Well, um, I think so.", Language::Ru, Mode::Light);
    assert_eq!(out, "Well, um, I think so.");
}

#[test]
fn light_collapses_repeated_filler_run_to_one_instance() {
    let out = fix("Так, эээ... эээ... начнём.", Language::Ru, Mode::Light);
    assert_eq!(out, "Так, эээ... начнём.");
}

#[test]
fn normal_removes_repeated_filler_run_entirely() {
    let out = fix("Так, эээ... эээ... начнём.", Language::Ru, Mode::Normal);
    assert_eq!(out, "Так, начнём.");
}

#[test]
fn empty_hesitation_composite_cleanup() {
    // ADR 0012 example: "Ну... эээ... да..." -> "Ну, да..."
    let out = fix("Ну... эээ... да...", Language::Ru, Mode::Light);
    assert_eq!(out, "Ну, да...");
}

#[test]
fn light_does_not_touch_false_starts() {
    let input = "Я... я думаю, что это норм.";
    let out = fix(input, Language::Ru, Mode::Light);
    assert_eq!(out, input);
}

#[test]
fn normal_collapses_false_start() {
    let out = fix("Я... я думаю, что это норм.", Language::Ru, Mode::Normal);
    assert_eq!(out, "Я думаю, что это норм.");
}

#[test]
fn aggressive_collapses_false_start_too() {
    let out = fix("Так... так поступим.", Language::Ru, Mode::Aggressive);
    assert_eq!(out, "Так поступим.");
}

#[test]
fn protected_phrase_blocks_false_start_removal() {
    // "ну да" is a protected discourse marker — must survive even though
    // "Ну... ну" superficially matches the false-start repeat pattern.
    let input = "Ну... ну да, я думаю.";
    let out = fix(input, Language::Ru, Mode::Aggressive);
    assert_eq!(out, input);
}

#[test]
fn protected_phrase_survives_light_and_off_too() {
    let input = "Ну конечно.";
    assert_eq!(fix(input, Language::Ru, Mode::Off), input);
    assert_eq!(fix(input, Language::Ru, Mode::Light), input);
    assert_eq!(fix(input, Language::Ru, Mode::Aggressive), input);
}

#[test]
fn mode_gating_produces_different_output_for_same_input() {
    let input = "Так, эээ... эээ... начнём. Я... я думаю, что это норм.";
    let off = fix(input, Language::Ru, Mode::Off);
    let light = fix(input, Language::Ru, Mode::Light);
    let normal = fix(input, Language::Ru, Mode::Normal);
    assert_eq!(off, input);
    assert_ne!(light, off);
    assert_ne!(normal, light);
    assert_ne!(normal, off);
}

#[test]
fn false_start_recapitalizes_surviving_word() {
    let out = fix("Я... я думаю.", Language::Ru, Mode::Normal);
    assert!(out.starts_with('Я'));
    assert_eq!(out, "Я думаю.");
}

#[test]
fn structural_guarantee_only_text_changes() {
    // FixResult carries text only; TextSpan exposes nothing else mutable —
    // this is enforced by vd-artifact, not re-tested here (see vd-artifact's
    // own test suite). This test just documents the expectation for readers.
    let result = fixer(Language::Ru, Mode::Light)
        .fix_text("эээ, тест", FixOptions::default())
        .unwrap();
    assert_eq!(result.text, "тест");
    assert!(result.changed);
}
