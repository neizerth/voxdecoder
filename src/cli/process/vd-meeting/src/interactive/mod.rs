//! Interactive wizard for meeting input selection (ADR 0017 Decision D).

use std::io::{self, BufRead, Cursor, Write};
use std::path::PathBuf;

use vd_pipeline::interactive::{MenuItem, MenuOutcome};

use crate::model::{InputRole, InputSource};

/// Run interactive meeting input wizard.
///
/// Collects media files from `working_dir`, classifies them via [`vd_classify`] heuristics,
/// shows a numbered menu for accept/edit-one/drop, auto-detects an optional context folder
/// (via [`crate::paths::resolve_context_dir`]), and returns confirmed inputs + context.
///
/// TTY auto-detected by the CLI layer (`validate_run`). If stdin is not a TTY or is closed,
/// the menu loop treats it as a quit — returns `Aborted`.
pub fn show_wizard(
    working_dir: Option<&std::path::Path>,
) -> Result<(Vec<InputSource>, Option<PathBuf>), String> {
    // Collect media files from working_dir (non-recursive, no dotfiles).
    let media_paths = collect_media_files(working_dir)?;
    if media_paths.is_empty() {
        return Err("no media files found in working directory".into());
    }

    // Classify each file by its basename.
    let classified = vd_classify::classify_inputs(&media_paths);

    // Build menu items: label = "{role} {name} [{gender}]"
    let mut menu_items: Vec<MenuItem<ClassifyState>> = classified
        .into_iter()
        .map(|c| {
            let gender_str = match c.gender {
                Some(g) => format!("{:?}", g),
                None => "?".to_string(),
            };
            let label = format!("{:?} {} [{}]", c.role, c.name, gender_str);
            MenuItem::new(ClassifyState::from_classified(c), label)
        })
        .collect();

    // Run the menu loop on stdin/stdout.
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut stdout = io::stdout();

    let outcome = vd_pipeline::interactive::run(
        &mut menu_items,
        |state| {
            let gender_str = match state.gender {
                Some(g) => format!("{:?}", g),
                None => "?".to_string(),
            };
            format!("{:?} {} [{}]", state.role, state.name, gender_str)
        },
        edit_input,
        &mut stdin_lock,
        &mut stdout,
    )
    .map_err(|e| format!("menu loop error: {}", e))?;

    if outcome == MenuOutcome::Aborted {
        return Err("wizard aborted by user".into());
    }

    // Convert accepted items to InputSource.
    let inputs: Vec<InputSource> = menu_items
        .into_iter()
        .map(|item| item.value.to_input_source())
        .collect();

    // Auto-detect context folder (offer to user).
    let first_path = inputs.first().map(|i| i.path.as_path());
    let mut context_path = crate::paths::resolve_context_dir(working_dir, first_path);

    if context_path.is_some() {
        eprint!("Found context folder. Add to inputs? (y/N): ");
        std::io::stderr().flush().ok();
        let mut buf = String::new();
        stdin_lock.read_line(&mut buf).ok();
        if !buf.trim().eq_ignore_ascii_case("y") {
            context_path = None;
        }
    }

    Ok((inputs, context_path))
}

/// Working state for one classified input during menu editing.
#[derive(Debug, Clone)]
struct ClassifyState {
    path: PathBuf,
    role: vd_classify::Role,
    name: String,
    gender: Option<vd_classify::Gender>,
}

impl ClassifyState {
    fn from_classified(c: vd_classify::ClassifiedInput) -> Self {
        Self {
            path: c.path,
            role: c.role,
            name: c.name,
            gender: c.gender,
        }
    }

    fn to_input_source(self) -> InputSource {
        InputSource {
            role: match self.role {
                vd_classify::Role::Room => InputRole::Room,
                vd_classify::Role::Participant => InputRole::Participant,
            },
            path: self.path,
            url: None,
            participant: match self.role {
                vd_classify::Role::Participant => Some(self.name),
                vd_classify::Role::Room => None,
            },
            purposes: vec![],
            subtitles: None,
            provider: None,
        }
    }
}

/// Edit callback for the menu loop. Parses user input like `role=participant,name=Ivan,gender=male`.
fn edit_input(
    state: &mut ClassifyState,
    input: &mut dyn BufRead,
    output: &mut dyn Write,
) -> io::Result<()> {
    writeln!(
        output,
        "Edit format: role=room|participant,name=...,gender=male|female|?"
    )?;
    writeln!(output, "Example: role=participant,name=Ivan,gender=male")?;
    write!(output, "> ")?;
    output.flush()?;

    let mut line = String::new();
    input.read_line(&mut line)?;
    let trimmed = line.trim();

    if !trimmed.is_empty() {
        if let Err(e) = parse_edit_string(trimmed, state) {
            writeln!(output, "parse error: {}", e)?;
        }
    }

    Ok(())
}

/// Parse edit string like `role=participant,name=Ivan,gender=male`.
fn parse_edit_string(s: &str, state: &mut ClassifyState) -> Result<(), String> {
    for part in s.split(',') {
        let part = part.trim();
        if let Some((key, val)) = part.split_once('=') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "role" => {
                    state.role = match val {
                        "room" | "merged" => vd_classify::Role::Room,
                        "participant" => vd_classify::Role::Participant,
                        other => return Err(format!("unknown role: {other}")),
                    };
                }
                "name" => {
                    state.name = val.to_string();
                }
                "gender" => {
                    state.gender = match val {
                        "male" | "m" => Some(vd_classify::Gender::Male),
                        "female" | "f" => Some(vd_classify::Gender::Female),
                        "?" | "unknown" | "none" => None,
                        other => return Err(format!("unknown gender: {other}")),
                    };
                }
                other => return Err(format!("unknown field: {other}")),
            }
        }
    }
    Ok(())
}

/// Collect media files from working_dir (non-recursive, files only, skip dotfiles).
fn collect_media_files(working_dir: Option<&std::path::Path>) -> Result<Vec<PathBuf>, String> {
    let dir = working_dir.ok_or("working directory required")?;
    let mut files = Vec::new();

    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if !name_str.starts_with('.') && is_media_file(&path) {
                            files.push(path);
                        }
                    }
                }
            }
        }
        Err(e) => return Err(format!("read_dir failed: {}", e)),
    }

    files.sort();
    Ok(files)
}

/// Check if a file has a media extension.
fn is_media_file(path: &std::path::Path) -> bool {
    const MEDIA_EXTS: &[&str] = &[
        "wav", "mp3", "m4a", "ogg", "opus", "flac", "aac", "wma", "aiff", "mp4", "mkv",
        "mov", "webm", "avi", "flv", "mpeg", "mpg",
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| MEDIA_EXTS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_edit_role_only() {
        let mut state = ClassifyState {
            path: PathBuf::from("test.wav"),
            role: vd_classify::Role::Participant,
            name: "Alice".into(),
            gender: None,
        };
        parse_edit_string("role=room", &mut state).unwrap();
        assert_eq!(state.role, vd_classify::Role::Room);
        assert_eq!(state.name, "Alice");
    }

    #[test]
    fn parse_edit_all_fields() {
        let mut state = ClassifyState {
            path: PathBuf::from("test.wav"),
            role: vd_classify::Role::Room,
            name: "mix".into(),
            gender: None,
        };
        parse_edit_string("role=participant,name=Игорь,gender=male", &mut state).unwrap();
        assert_eq!(state.role, vd_classify::Role::Participant);
        assert_eq!(state.name, "Игорь");
        assert_eq!(state.gender, Some(vd_classify::Gender::Male));
    }

    #[test]
    fn parse_edit_gender_variants() {
        let mut state = ClassifyState {
            path: PathBuf::from("test.wav"),
            role: vd_classify::Role::Participant,
            name: "test".into(),
            gender: None,
        };
        parse_edit_string("gender=f", &mut state).unwrap();
        assert_eq!(state.gender, Some(vd_classify::Gender::Female));

        parse_edit_string("gender=?", &mut state).unwrap();
        assert_eq!(state.gender, None);
    }

    #[test]
    fn parse_edit_invalid_role() {
        let mut state = ClassifyState {
            path: PathBuf::from("test.wav"),
            role: vd_classify::Role::Participant,
            name: "Alice".into(),
            gender: None,
        };
        let err = parse_edit_string("role=unknown", &mut state);
        assert!(err.is_err());
    }

    #[test]
    fn parse_edit_unknown_field() {
        let mut state = ClassifyState {
            path: PathBuf::from("test.wav"),
            role: vd_classify::Role::Participant,
            name: "Alice".into(),
            gender: None,
        };
        let err = parse_edit_string("unknown=value", &mut state);
        assert!(err.is_err());
    }

    #[test]
    fn classify_state_room_to_input_source() {
        let state = ClassifyState {
            path: PathBuf::from("mix.wav"),
            role: vd_classify::Role::Room,
            name: "mix".into(),
            gender: None,
        };
        let input = state.to_input_source();
        assert_eq!(input.role, InputRole::Room);
        assert_eq!(input.participant, None);
        assert_eq!(input.path, PathBuf::from("mix.wav"));
    }

    #[test]
    fn classify_state_participant_to_input_source() {
        let state = ClassifyState {
            path: PathBuf::from("alice.wav"),
            role: vd_classify::Role::Participant,
            name: "alice".into(),
            gender: Some(vd_classify::Gender::Female),
        };
        let input = state.to_input_source();
        assert_eq!(input.role, InputRole::Participant);
        assert_eq!(input.participant, Some("alice".into()));
        assert_eq!(input.path, PathBuf::from("alice.wav"));
    }

    #[test]
    fn is_media_file_recognizes_common_formats() {
        assert!(is_media_file(PathBuf::from("audio.wav").as_path()));
        assert!(is_media_file(PathBuf::from("song.mp3").as_path()));
        assert!(is_media_file(PathBuf::from("video.mp4").as_path()));
        assert!(is_media_file(PathBuf::from("AUDIO.WAV").as_path()));
    }

    #[test]
    fn is_media_file_rejects_non_media() {
        assert!(!is_media_file(PathBuf::from("readme.txt").as_path()));
        assert!(!is_media_file(PathBuf::from("doc.pdf").as_path()));
        assert!(!is_media_file(PathBuf::from("script.sh").as_path()));
    }

    #[test]
    fn menu_loop_with_mocked_input() {
        use std::io::Cursor;
        let mut items = vec![MenuItem::new(
            ClassifyState {
                path: PathBuf::from("test.wav"),
                role: vd_classify::Role::Participant,
                name: "alice".into(),
                gender: Some(vd_classify::Gender::Female),
            },
            "participant alice [Female]".to_string(),
        )];

        let mut input = Cursor::new(b"y\n".to_vec());
        let mut output = Vec::new();

        let outcome = vd_pipeline::interactive::run(
            &mut items,
            |s| format!("{:?} {}", s.role, s.name),
            |_, _, _| Ok(()),
            &mut input,
            &mut output,
        )
        .unwrap();

        assert_eq!(outcome, MenuOutcome::Accepted);
        assert_eq!(items.len(), 1);
    }
}
