//! Per-file orchestration: `path` → [`crate::ClassifiedInput`].
//!
//! **STUB — implementation tracked separately (ADR 0017 P1-B).** Composes
//! [`crate::strip_basename_noise`], [`crate::is_mix_token`], and [`crate::infer_gender`] — no
//! new heuristics belong in this function itself, only wiring.
//!
//! Scope note: this assigns `role` / `name` / `gender` per file only. It does **not** decide
//! diarization mode when both a mix and participant tracks are present — that is the
//! **Mix + tracks** choice from skill.md, a UX decision the `--interactive` wizard (ADR 0017
//! Decision D) makes after showing the user this crate's proposal, not something baked into
//! the classification itself.

use std::path::{Path, PathBuf};

use crate::{infer_gender, is_mix_token, strip_basename_noise, ClassifiedInput, Role};

/// Propose a classification for each of `paths`.
///
/// For each path: take `Path::file_stem()`, run [`crate::strip_basename_noise`] to get a
/// candidate name, check [`crate::is_mix_token`] on the (lowercased) result to decide
/// `Role::Room` vs `Role::Participant`, and for `Role::Participant` run
/// [`crate::infer_gender`] on the name. Order of `paths` is preserved in the output.
///
/// Per skill.md: "Prefer the original script from the filename... never transliterate
/// Cyrillic → Latin for `participant` or display" — the `name` field must be the cleaned
/// stem's original script/casing, not an ASCII slug, when the source name is Cyrillic.
pub fn classify_inputs(paths: &[PathBuf]) -> Vec<ClassifiedInput> {
    paths
        .iter()
        .filter_map(|path| classify_one(path))
        .collect()
}

fn classify_one(path: &Path) -> Option<ClassifiedInput> {
    let stem = path.file_stem()?.to_str()?;
    let name = strip_basename_noise(stem);
    let role = if is_mix_token(&name) {
        Role::Room
    } else {
        Role::Participant
    };
    let gender = if role == Role::Participant {
        infer_gender(&name)
    } else {
        None
    };
    Some(ClassifiedInput {
        path: path.to_path_buf(),
        role,
        name,
        gender,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_mix_from_participants() {
        let paths = vec![
            PathBuf::from("meeting_mix_2024-01-15.wav"),
            PathBuf::from("Игорь.wav"),
        ];
        let out = classify_inputs(&paths);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].role, Role::Room);
        assert_eq!(out[1].role, Role::Participant);
        assert_eq!(out[1].name, "Игорь");
    }
}
