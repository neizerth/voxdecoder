//! Gender inference from a given name.
//!
//! **STUB — implementation tracked separately (ADR 0017 P1-B).**
//!
//! Unlike [`crate::is_mix_token`], `skills/vd-meeting/skill.md`'s **Gender** section does
//! *not* enumerate a finite name → gender table — it delegates to "common language
//! conventions (RU/EN and other languages you know)", i.e. an LLM's background knowledge.
//! There is no prose table to transcribe here. Implementing this deterministically needs an
//! actual bundled name → gender lookup (a reasonably sized list of common Russian/English
//! given names is enough to cover the common case; anything not in the list must return
//! `None`, never a guess) — that list does not exist yet and is part of this stub's work, not
//! just a straight port like [`crate::is_mix_token`].
//!
//! Rule to preserve from skill.md: return `None` (do not guess) for ambiguous/unisex
//! nicknames (`Alex`, `Саша`, `Женя` are skill.md's own examples) — callers must ask the user
//! rather than have this function invent an answer. Never overridden by anything upstream of
//! an explicit user-stated gender (that precedence lives in the caller, not here).

/// Inferred gender, when confident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
}

/// Infer gender from a given name (already cleaned, e.g. via [`crate::strip_basename_noise`]).
///
/// Returns `None` when not confidently inferable — see module docs. Must never guess on
/// ambiguous/unisex names.
pub fn infer_gender(given_name: &str) -> Option<Gender> {
    if given_name.is_empty() {
        return None;
    }
    let lower = given_name.to_lowercase();

    // Russian female names (common -а, -я, -ь endings for female)
    if is_female_russian(&lower) {
        return Some(Gender::Female);
    }
    if is_male_russian(&lower) {
        return Some(Gender::Male);
    }

    // English names (explicit set; no heuristics to avoid false positives)
    if let Some(gender) = lookup_english(&lower) {
        return Some(gender);
    }

    None
}

fn is_female_russian(name: &str) -> bool {
    const FEMALE: &[&str] = &[
        "мария", "нина", "галина", "елена", "ольга", "анна", "таня", "юлия", "софья",
        "наталья", "людмила", "ирина", "виктория", "алиса", "наталия", "яна", "дарья",
        "полина", "татьяна", "валентина", "маргарита", "светлана", "евгения", "станислава",
    ];
    FEMALE.contains(&name)
}

fn is_male_russian(name: &str) -> bool {
    const MALE: &[&str] = &[
        "игорь", "владимир", "иван", "сергей", "алексей", "петр", "павел", "михаил",
        "виктор", "дмитрий", "степан", "константин", "юрий", "борис", "александр",
        "николай", "артем", "андрей", "василий", "владислав", "григорий", "денис",
        "евгений", "илья", "кирилл", "леонид", "максим", "станислав", "сергей", "юрий",
    ];
    MALE.contains(&name)
}

fn lookup_english(name: &str) -> Option<Gender> {
    const FEMALE: &[&str] = &[
        "mary", "susan", "karen", "nancy", "lisa", "betty", "margaret", "sandra", "ashley",
        "dorothy", "catherine", "elizabeth", "anna", "deborah", "jessica", "sarah", "joyce",
        "diane", "virginia", "joyce", "victoria", "grace", "barbara", "ruth", "ann",
        "christine", "janet", "catherine", "maria", "heather", "charlotte", "alice",
        "helen", "edith", "rose", "jean", "diane", "julie", "joyce",
    ];
    const MALE: &[&str] = &[
        "john", "michael", "david", "james", "robert", "richard", "thomas", "charles",
        "peter", "paul", "daniel", "george", "steven", "edward", "brian", "ronald", "anthony",
        "frank", "ryan", "gary", "nicholas", "eric", "jonathan", "stephen", "larry", "justin",
        "scott", "brandon", "benjamin", "samuel", "raymond", "gregory", "alexander", "arthur",
    ];

    if FEMALE.contains(&name) {
        return Some(Gender::Female);
    }
    if MALE.contains(&name) {
        return Some(Gender::Male);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unambiguous_names_resolve() {
        assert_eq!(infer_gender("Игорь"), Some(Gender::Male));
        assert_eq!(infer_gender("Мария"), Some(Gender::Female));
    }

    #[test]
    fn ambiguous_nicknames_return_none() {
        // skill.md's own examples of names it explicitly refuses to guess on.
        assert_eq!(infer_gender("Alex"), None);
        assert_eq!(infer_gender("Саша"), None);
        assert_eq!(infer_gender("Женя"), None);
    }
}
