//! Platform and project path helpers for VoxDecoder CLIs.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

/// Default project assets directory name (dot-prefixed so it stays out of the way).
pub const DEFAULT_PROJECT_DIR_NAME: &str = ".voxdecoder";

/// Subdirectory under `.voxdecoder/` for Job intermediates (prepared media, transcripts, fixed text).
pub const DEFAULT_WORK_SUBDIR: &str = "work";

/// Process-env / project-env key for overriding the project assets directory.
pub const ENV_PROJECT_DIR: &str = "VD_PROJECT_DIR";

/// Config file path: `$ENV` if set, else `{config_dir}/config.toml` for `app`.
pub fn config_path(app: &str, env_var: &str) -> PathBuf {
    if let Ok(p) = env::var(env_var) {
        return PathBuf::from(p);
    }
    ProjectDirs::from("", "", app)
        .map(|d| d.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from(format!("{app}-config.toml")))
}

/// Cache subdirectory: `$ENV` if set, else `{cache_dir}/{subdir}` for `app`.
pub fn cache_dir(app: &str, env_var: &str, subdir: &str) -> PathBuf {
    if let Ok(p) = env::var(env_var) {
        return PathBuf::from(p);
    }
    ProjectDirs::from("", "", app)
        .map(|d| d.cache_dir().join(subdir))
        .unwrap_or_else(|| PathBuf::from(subdir))
}

/// Resolve the project assets directory for writing (e.g. `vd-assets -o` default).
///
/// Priority:
/// 1. `$VD_PROJECT_DIR`
/// 2. `VD_PROJECT_DIR=` in `.voxdecoder/env` or `.env` (walk up from `start`)
/// 3. nearest `.voxdecoder/` directory walking up from `start`
/// 4. `{cwd}/.voxdecoder`
pub fn project_dir(start: &Path) -> PathBuf {
    if let Some(p) = resolve_project_dir(start) {
        return p;
    }
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(DEFAULT_PROJECT_DIR_NAME)
}

/// Job intermediates for an input file: `{input_parent}/.voxdecoder/work`.
///
/// Keeps prepared media / transcripts / `.fixed` outputs out of the source folder.
/// Project assets (`md/`, `terms.yml`) stay in `.voxdecoder/` itself.
///
/// If `input` is already under `.voxdecoder/work/`, returns that work directory
/// (does not nest another `.voxdecoder/work`).
pub fn work_dir_for_input(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    if is_work_dir(parent) {
        return parent.to_path_buf();
    }
    parent
        .join(DEFAULT_PROJECT_DIR_NAME)
        .join(DEFAULT_WORK_SUBDIR)
}

fn is_work_dir(path: &Path) -> bool {
    path.file_name().and_then(|s| s.to_str()) == Some(DEFAULT_WORK_SUBDIR)
        && path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            == Some(DEFAULT_PROJECT_DIR_NAME)
}

/// Like [`project_dir`], but only when the directory already exists.
/// Used as the default `--context` / `--terms` for `vd-fix-*`.
pub fn project_dir_if_present(start: &Path) -> Option<PathBuf> {
    let p = resolve_project_dir(start)?;
    p.is_dir().then_some(p)
}

fn resolve_project_dir(start: &Path) -> Option<PathBuf> {
    if let Ok(p) = env::var(ENV_PROJECT_DIR) {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    let mut dir = if start.is_file() {
        start.parent().map(Path::to_path_buf)
    } else if start.as_os_str().is_empty() {
        env::current_dir().ok()
    } else {
        Some(start.to_path_buf())
    };

    while let Some(d) = dir {
        if let Some(from_env) = read_project_dir_key(&d.join(DEFAULT_PROJECT_DIR_NAME).join("env"))
            .or_else(|| read_project_dir_key(&d.join(".env")))
        {
            return Some(resolve_against(&d, &from_env));
        }
        let candidate = d.join(DEFAULT_PROJECT_DIR_NAME);
        if candidate.is_dir() {
            return Some(candidate);
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    None
}

fn resolve_against(base: &Path, raw: &str) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        p
    } else {
        base.join(p)
    }
}

/// Read `VD_PROJECT_DIR=` from a simple env file (`KEY=VALUE`, `#` comments, optional `export`).
fn read_project_dir_key(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != ENV_PROJECT_DIR {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'').trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn finds_dot_voxdecoder_near_input() {
        let _g = ENV_LOCK.lock().unwrap();
        env::remove_var(ENV_PROJECT_DIR);
        let dir = tempfile::TempDir::new().unwrap();
        let assets = dir.path().join(DEFAULT_PROJECT_DIR_NAME);
        fs::create_dir(&assets).unwrap();
        let input = dir.path().join("meeting.txt");
        fs::write(&input, "x").unwrap();

        assert_eq!(
            project_dir_if_present(&input).as_deref(),
            Some(assets.as_path())
        );
    }

    #[test]
    fn env_file_overrides_default_name() {
        let _g = ENV_LOCK.lock().unwrap();
        env::remove_var(ENV_PROJECT_DIR);
        let dir = tempfile::TempDir::new().unwrap();
        let custom = dir.path().join("knowledge");
        fs::create_dir(&custom).unwrap();
        let dot = dir.path().join(DEFAULT_PROJECT_DIR_NAME);
        fs::create_dir(&dot).unwrap();
        fs::write(dot.join("env"), "VD_PROJECT_DIR=./knowledge\n").unwrap();
        let input = dir.path().join("a.txt");
        fs::write(&input, "x").unwrap();

        assert_eq!(
            project_dir_if_present(&input).as_deref(),
            Some(custom.as_path())
        );
    }

    #[test]
    fn work_dir_is_under_dot_voxdecoder() {
        let dir = tempfile::TempDir::new().unwrap();
        let input = dir.path().join("meeting.ogg");
        fs::write(&input, "x").unwrap();
        assert_eq!(
            work_dir_for_input(&input),
            dir.path()
                .join(DEFAULT_PROJECT_DIR_NAME)
                .join(DEFAULT_WORK_SUBDIR)
        );
    }

    #[test]
    fn work_dir_does_not_nest() {
        let dir = tempfile::TempDir::new().unwrap();
        let work = dir
            .path()
            .join(DEFAULT_PROJECT_DIR_NAME)
            .join(DEFAULT_WORK_SUBDIR);
        fs::create_dir_all(&work).unwrap();
        let input = work.join("meeting.prepared.mp3");
        fs::write(&input, "x").unwrap();
        assert_eq!(work_dir_for_input(&input), work);
    }
}
