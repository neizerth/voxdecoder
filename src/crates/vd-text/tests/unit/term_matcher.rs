//! `vd_text::term_matcher` — Aho-Corasick-backed terminology matching.

use vd_text::term_matcher::TermMatcher;

fn entries(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(v, c)| (v.to_string(), c.to_string()))
        .collect()
}

#[test]
fn finds_single_occurrence() {
    let m = TermMatcher::new(entries(&[("JS Fidls", "JSFiddle")])).unwrap();
    let matches = m.find_all("check out JS Fidls for a demo");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].canonical, "JSFiddle");
}

#[test]
fn replace_all_swaps_every_occurrence() {
    let m = TermMatcher::new(entries(&[("RBNB", "Airbnb"), ("Avisales", "Aviasales")])).unwrap();
    let out = m.replace_all("We used RBNB and also Avisales for the trip.");
    assert_eq!(out, "We used Airbnb and also Aviasales for the trip.");
}

#[test]
fn no_match_leaves_text_untouched() {
    let m = TermMatcher::new(entries(&[("RBNB", "Airbnb")])).unwrap();
    let out = m.replace_all("nothing to see here");
    assert_eq!(out, "nothing to see here");
    assert!(m.find_all("nothing to see here").is_empty());
}

#[test]
fn leftmost_longest_wins_on_overlapping_patterns() {
    let m = TermMatcher::new(entries(&[
        ("Git", "git-short"),
        ("GitHub", "GitHub-canonical"),
    ]))
    .unwrap();
    let matches = m.find_all("we use GitHub daily");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].canonical, "GitHub-canonical");
}

#[test]
fn later_entry_for_same_variant_wins() {
    let m = TermMatcher::new(entries(&[("RBNB", "wrong"), ("RBNB", "Airbnb")])).unwrap();
    let matches = m.find_all("RBNB");
    assert_eq!(matches[0].canonical, "Airbnb");
}

#[test]
fn case_sensitive_by_default() {
    let m = TermMatcher::new(entries(&[("GraphQL", "GraphQL")])).unwrap();
    assert!(m.find_all("graphql is different casing").is_empty());
    assert_eq!(m.find_all("GraphQL matches").len(), 1);
}

#[test]
fn ascii_case_insensitive_variant_matches_any_casing() {
    let m = TermMatcher::new_ascii_case_insensitive(entries(&[("GraphQL", "GraphQL")])).unwrap();
    assert_eq!(m.find_all("graphql GRAPHQL GraphQL").len(), 3);
}

#[test]
fn cyrillic_terms_match_exactly() {
    let m = TermMatcher::new(entries(&[("рест апи", "REST API")])).unwrap();
    let out = m.replace_all("мы используем рест апи для интеграции");
    assert_eq!(out, "мы используем REST API для интеграции");
}
