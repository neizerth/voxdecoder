//! Staged deterministic pipeline (ADR 0010): rule/stage primitives, spacing,
//! punctuation.

use std::collections::HashMap;
use std::sync::Arc;

use vd_fix_asr::asr::rule::{Confidence, RuleCategory, RuleHit};
use vd_fix_asr::asr::stages::{
    alphabet, dictionary, duplicate, merge_split, punctuation, spacing, ConfidencePolicy, Pipeline,
    Stage, StageId, StageOutcome,
};
use vd_fix_asr::types::SpanId;

#[test]
fn report_keys_are_stable() {
    assert_eq!(RuleCategory::Spacing.report_key(), "spacing");
    assert_eq!(RuleCategory::Dictionary.report_key(), "dictionary");
}

#[test]
fn rule_hit_carries_span_id_once_attached() {
    let hit = RuleHit {
        category: RuleCategory::Punctuation,
        confidence: Confidence::Certain,
        rule_id: "punctuation:ellipsis",
        span_id: None,
        before: "...".to_string(),
        after: "…".to_string(),
    };
    assert_eq!(hit.span_id, None);
    let attached = RuleHit {
        span_id: Some(SpanId(3)),
        ..hit
    };
    assert_eq!(attached.span_id, Some(SpanId(3)));
}

struct Noop;
impl Stage for Noop {
    fn id(&self) -> StageId {
        StageId::Spacing
    }
    fn run(&self, input: &str, _policy: &ConfidencePolicy) -> StageOutcome {
        StageOutcome {
            text: input.to_string(),
            hits: Vec::new(),
        }
    }
}

#[test]
fn empty_pipeline_is_identity() {
    let pipeline = Pipeline::default();
    let out = pipeline.run("hello  world", &ConfidencePolicy::default());
    assert_eq!(out.text, "hello  world");
    assert!(out.hits.is_empty());
}

#[test]
fn single_noop_stage_is_identity() {
    let pipeline = Pipeline::new(vec![Box::new(Noop)]);
    let out = pipeline.run("unchanged", &ConfidencePolicy::default());
    assert_eq!(out.text, "unchanged");
}

#[test]
fn default_policy_allows_certain_and_likely_not_unsafe() {
    let policy = ConfidencePolicy::default();
    assert!(policy.allows(Confidence::Certain));
    assert!(policy.allows(Confidence::Likely));
    assert!(!policy.allows(Confidence::Unsafe));
}

#[test]
fn strict_policy_allows_certain_only() {
    let policy = ConfidencePolicy {
        certain_only: true,
        allow_unsafe: false,
    };
    assert!(policy.allows(Confidence::Certain));
    assert!(!policy.allows(Confidence::Likely));
    assert!(!policy.allows(Confidence::Unsafe));
}

#[test]
fn aggressive_policy_allows_unsafe() {
    let policy = ConfidencePolicy {
        certain_only: false,
        allow_unsafe: true,
    };
    assert!(policy.allows(Confidence::Unsafe));
}

mod spacing_stage {
    use super::{spacing, ConfidencePolicy, Stage};

    #[test]
    fn collapses_multiple_spaces() {
        let out = spacing::stage().run("hello   world", &ConfidencePolicy::default());
        assert_eq!(out.text, "hello world");
        assert_eq!(out.hits.len(), 1);
    }

    #[test]
    fn converts_tabs_to_space() {
        let out = spacing::stage().run("a\tb", &ConfidencePolicy::default());
        assert_eq!(out.text, "a b");
    }

    #[test]
    fn normalizes_crlf_and_lone_cr() {
        let out = spacing::stage().run("a\r\nb\rc", &ConfidencePolicy::default());
        assert_eq!(out.text, "a\nb\nc");
    }

    #[test]
    fn leaves_single_space_and_newlines_alone() {
        let out = spacing::stage().run("hello world\nsecond line", &ConfidencePolicy::default());
        assert_eq!(out.text, "hello world\nsecond line");
        assert!(out.hits.is_empty());
    }

    #[test]
    fn tab_then_space_run_collapses_together() {
        let out = spacing::stage().run("a\t  b", &ConfidencePolicy::default());
        assert_eq!(out.text, "a b");
    }
}

mod punctuation_stage {
    use super::{punctuation, ConfidencePolicy, Stage};

    #[test]
    fn collapses_four_dots_to_ellipsis() {
        let out = punctuation::stage().run("подожди....", &ConfidencePolicy::default());
        assert_eq!(out.text, "подожди…");
    }

    #[test]
    fn keeps_two_dots_untouched() {
        let out = punctuation::stage().run("wait..", &ConfidencePolicy::default());
        assert_eq!(out.text, "wait..");
    }

    #[test]
    fn collapses_duplicate_exclamation() {
        let out = punctuation::stage().run("вот это да!!!", &ConfidencePolicy::default());
        assert_eq!(out.text, "вот это да!");
    }

    #[test]
    fn removes_space_before_comma() {
        let out = punctuation::stage().run("Да , конечно", &ConfidencePolicy::default());
        assert_eq!(out.text, "Да, конечно");
    }

    #[test]
    fn removes_multiple_spaces_before_period() {
        let out = punctuation::stage().run("готово   .", &ConfidencePolicy::default());
        assert_eq!(out.text, "готово.");
    }

    #[test]
    fn leaves_clean_punctuation_alone() {
        let out = punctuation::stage().run("Привет, как дела?", &ConfidencePolicy::default());
        assert_eq!(out.text, "Привет, как дела?");
        assert!(out.hits.is_empty());
    }
}

mod duplicate_stage {
    use super::{duplicate, ConfidencePolicy, Stage};

    #[test]
    fn collapses_adjacent_word_duplicate() {
        let out = duplicate::stage().run("вот вот это круто", &ConfidencePolicy::default());
        assert_eq!(out.text, "вот это круто");
    }

    #[test]
    fn collapses_triple_adjacent_duplicate() {
        let out = duplicate::stage().run("вот вот вот", &ConfidencePolicy::default());
        assert_eq!(out.text, "вот");
    }

    #[test]
    fn collapses_multiword_adjacent_duplicate() {
        let out = duplicate::stage().run("эти эти стандарты", &ConfidencePolicy::default());
        assert_eq!(out.text, "эти стандарты");
    }

    #[test]
    fn collapses_i_filler_repeat_fully() {
        let out = duplicate::stage().run("ииии не знаю", &ConfidencePolicy::default());
        assert_eq!(out.text, "и не знаю");
    }

    #[test]
    fn collapses_e_filler_repeat_to_two() {
        let out = duplicate::stage().run("ээээ подожди", &ConfidencePolicy::default());
        assert_eq!(out.text, "ээ подожди");
    }

    #[test]
    fn collapses_within_token_doubling_by_default() {
        let out = duplicate::stage().run("каккак сделать", &ConfidencePolicy::default());
        assert_eq!(out.text, "как сделать");
    }

    #[test]
    fn strict_policy_keeps_within_token_doubling() {
        let policy = ConfidencePolicy {
            certain_only: true,
            allow_unsafe: false,
        };
        let out = duplicate::stage().run("каккак сделать", &policy);
        assert_eq!(out.text, "каккак сделать");
        assert_eq!(
            out.hits.len(),
            1,
            "hit still reported even though not applied"
        );
    }

    #[test]
    fn does_not_collapse_short_reduplicated_real_word() {
        let out = duplicate::stage().run("мама дома", &ConfidencePolicy::default());
        assert_eq!(out.text, "мама дома");
    }

    #[test]
    fn leaves_clean_text_alone() {
        let out = duplicate::stage().run("привет, как дела", &ConfidencePolicy::default());
        assert_eq!(out.text, "привет, как дела");
        assert!(out.hits.is_empty());
    }
}

mod alphabet_stage {
    use super::{alphabet, ConfidencePolicy, Stage};

    #[test]
    fn normalizes_stray_cyrillic_in_latin_dominant_word() {
        // "МAC": Cyrillic М + Latin A, C → Latin dominant, М is a known
        // homoglyph of M.
        let out = alphabet::stage().run("МAC адрес", &ConfidencePolicy::default());
        assert_eq!(out.text, "MAC адрес");
    }

    #[test]
    fn normalizes_stray_latin_in_cyrillic_dominant_word() {
        // "СOН": Latin O amid Cyrillic С, Н → Cyrillic dominant.
        let out = alphabet::stage().run("СOН приснился", &ConfidencePolicy::default());
        assert_eq!(out.text, "СОН приснился");
    }

    #[test]
    fn abstains_on_unresolvable_minority_letter() {
        // Ж has no Latin homoglyph in the table — leave untouched entirely.
        let out = alphabet::stage().run("GitЖub", &ConfidencePolicy::default());
        assert_eq!(out.text, "GitЖub");
        assert!(out.hits.is_empty());
    }

    #[test]
    fn abstains_on_tied_script_counts() {
        let out = alphabet::stage().run("АB", &ConfidencePolicy::default());
        assert_eq!(out.text, "АB");
        assert!(out.hits.is_empty());
    }

    #[test]
    fn leaves_single_script_words_alone() {
        let out = alphabet::stage().run("привет hello", &ConfidencePolicy::default());
        assert_eq!(out.text, "привет hello");
        assert!(out.hits.is_empty());
    }
}

mod merge_split_stage {
    use super::{merge_split, ConfidencePolicy, Stage};

    #[test]
    fn splits_known_merged_word() {
        let out = merge_split::stage().run("этотоже нормально", &ConfidencePolicy::default());
        assert_eq!(out.text, "это тоже нормально");
    }

    #[test]
    fn splits_and_restores_leading_case() {
        let out = merge_split::stage().run("Этотоже", &ConfidencePolicy::default());
        assert_eq!(out.text, "Это тоже");
    }

    #[test]
    fn merges_known_word_pair() {
        let out = merge_split::stage().run("этот дата сет большой", &ConfidencePolicy::default());
        assert_eq!(out.text, "этот датасет большой");
    }

    #[test]
    fn does_not_merge_across_punctuation() {
        let out = merge_split::stage().run("дата, сет", &ConfidencePolicy::default());
        assert_eq!(out.text, "дата, сет");
    }

    #[test]
    fn leaves_unrelated_text_alone() {
        let out = merge_split::stage().run("привет, как дела", &ConfidencePolicy::default());
        assert_eq!(out.text, "привет, как дела");
        assert!(out.hits.is_empty());
    }
}

mod dictionary_stage {
    use super::{dictionary, Arc, ConfidencePolicy, HashMap, Stage};

    #[test]
    fn applies_static_lookup_case_insensitively() {
        let mut map = HashMap::new();
        map.insert("гитхап".to_string(), "гитхаб".to_string());
        let out =
            dictionary::stage(Arc::new(map)).run("Гитхап рулит", &ConfidencePolicy::default());
        assert_eq!(out.text, "Гитхаб рулит");
    }

    #[test]
    fn leaves_unknown_words_alone() {
        let map = HashMap::new();
        let out = dictionary::stage(Arc::new(map)).run("привет мир", &ConfidencePolicy::default());
        assert_eq!(out.text, "привет мир");
        assert!(out.hits.is_empty());
    }
}
