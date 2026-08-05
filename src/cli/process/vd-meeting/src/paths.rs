
//! Platform paths for `vd-meeting`.

use std::path::{Path, PathBuf};

const ENV_CONFIG: &str = "VD_MEETING_CONFIG";
const APP: &str = "vd-meeting";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path(APP, ENV_CONFIG)
}

/// Auto-detect a context folder when none is explicitly provided.
///
/// Search order (ADR 0017 Decision C):
/// 1. If explicit context path given, return it (caller responsibility to pass it here).
/// 2. Check working_dir/context/ (preferred: same project).
/// 3. If any input path present, check input_parent/context/ (first input's directory).
/// 4. Return None if no context folder found (context is optional).
///
/// Intended for `vd-meeting --interactive`: before showing the user the file list, auto-detect
/// context if available, then offer it as part of the wizard.
pub fn resolve_context_dir(
    working_dir: Option<&Path>,
    first_input: Option<&Path>,
) -> Option<PathBuf> {
    // Check working_dir/context/ first
    if let Some(wd) = working_dir {
        let context = wd.join("context");
        if context.is_dir() {
            return Some(context);
        }
    }

    // Check input's parent directory / context
    if let Some(input) = first_input {
        if let Some(parent) = input.parent() {
            let context = parent.join("context");
            if context.is_dir() {
                return Some(context);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn finds_context_in_working_dir() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("context")).unwrap();
        let found = resolve_context_dir(Some(temp.path()), None);
        assert_eq!(found, Some(temp.path().join("context")));
    }

    #[test]
    fn finds_context_in_input_parent() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("context")).unwrap();
        let input = temp.path().join("audio.wav");
        let found = resolve_context_dir(None, Some(&input));
        assert_eq!(found, Some(temp.path().join("context")));
    }

    #[test]
    fn prefers_working_dir_over_input_parent() {
        let temp = TempDir::new().unwrap();
        fs::create_dir(temp.path().join("context")).unwrap();
        let input = temp.path().join("audio.wav");
        let found = resolve_context_dir(Some(temp.path()), Some(&input));
        assert_eq!(found, Some(temp.path().join("context")));
    }

    #[test]
    fn returns_none_when_no_context_found() {
        let temp = TempDir::new().unwrap();
        let input = temp.path().join("audio.wav");
        let found = resolve_context_dir(Some(temp.path()), Some(&input));
        assert_eq!(found, None);
    }

    #[test]
    fn returns_none_when_all_none() {
        let found = resolve_context_dir(None, None);
        assert_eq!(found, None);
    }
}
