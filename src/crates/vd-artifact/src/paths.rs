//! Platform and project path helpers for VoxDecoder CLIs.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use directories::ProjectDirs;

/// Process-env override for the global cache root (mirrors `vdctl`'s `VD_HOME`).
pub const ENV_HOME: &str = "VD_HOME";

/// Subdirectory of `$VD_HOME` holding the content-addressed Job cache (ADR 0017).
pub const CACHE_DIR_NAME: &str = "cache";

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

/// Platform home (data root) — mirrors `vdctl::paths::home_dir()`: `$VD_HOME` if set,
/// else the OS `ProjectDirs` data dir for the `voxdecoder` app.
fn platform_home_dir() -> PathBuf {
    if let Ok(p) = env::var(ENV_HOME) {
        let t = p.trim();
        if !t.is_empty() {
            return PathBuf::from(t);
        }
    }
    ProjectDirs::from("", "", "voxdecoder")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".voxdecoder"))
}

/// Mint a new Job run id: `job-{nanos:x}-{pid:x}`.
///
/// Shared by `vd-srv` (`JobRecord.id`, minted on submit) and local `vd-meeting`/`vd-pipeline`
/// CLI runs with no Runtime involved (minted before `resolve_job`) — both produce ids in the
/// same format (ADR 0017 Decision B). This id is a *run* identity (one per submit/attempt),
/// distinct from a [`content_hash_key`] or [`job_cache_dir`] cache key: retries of the same
/// meeting mint a fresh run id each time but are expected to be resumed against the same
/// cache key by the caller re-supplying it.
pub fn new_job_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    format!("job-{nanos:x}-{pid:x}")
}

/// Global, content-addressed Job cache root: `$VD_HOME/cache` (ADR 0017).
///
/// Sibling to `$VD_HOME/models`, `$VD_HOME/skills`, `$VD_HOME/bundles` — the one platform-data
/// root every binary (Workspace or Installed) already agrees on. Nothing under this tree is
/// written next to user media / project files.
pub fn job_cache_root() -> PathBuf {
    platform_home_dir().join(CACHE_DIR_NAME)
}

/// Cache directory for a specific key: `$VD_HOME/cache/{key}`.
///
/// `key` is either a [`content_hash_key`] (single-input / audio Jobs) or a Job `job_id`
/// (multi-input / meeting Jobs) — see ADR 0017 Decision B.
pub fn job_cache_dir(key: &str) -> PathBuf {
    job_cache_root().join(key)
}

/// BLAKE3 content hash of a file, hex-encoded — the cache key for single-input Jobs.
///
/// Streams the file instead of reading it fully into memory: hashing cost is negligible
/// next to multi-minute ASR wall time, but multi-GB video inputs should not be loaded whole.
pub fn content_hash_key(path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

/// Temp sibling path for atomically producing `final_path`: `{parent}/{file_name}.tmp-{pid}`.
///
/// Callers (subprocess invocations, direct writers) write to this path, then call
/// [`finalize_atomic`] to make the result visible under `final_path`. A reader that only
/// ever looks at `final_path` never observes a partially written file — `rename()` is atomic
/// on the same filesystem, so a process crashing mid-write leaves only an orphaned `.tmp-*`
/// file, never a corrupt `final_path` a later resume could mistake for a completed step.
pub fn atomic_temp_path(final_path: &Path) -> PathBuf {
    let name = final_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("out");
    let parent = final_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{name}.tmp-{}", std::process::id()))
}

/// Make `tmp_path` visible as `final_path` (rename). Ensures `final_path`'s parent dir exists.
///
/// Only call this after `tmp_path` was fully written and closed — the rename itself is the
/// atomic step; this function does not flush or sync the file.
pub fn finalize_atomic(tmp_path: &Path, final_path: &Path) -> io::Result<()> {
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(tmp_path, final_path)
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

    #[test]
    fn job_cache_root_honors_vd_home() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        env::set_var(ENV_HOME, dir.path());
        assert_eq!(job_cache_root(), dir.path().join(CACHE_DIR_NAME));
        assert_eq!(job_cache_dir("abc123"), dir.path().join(CACHE_DIR_NAME).join("abc123"));
        env::remove_var(ENV_HOME);
    }

    #[test]
    fn content_hash_key_is_deterministic_and_content_sensitive() {
        let dir = tempfile::TempDir::new().unwrap();
        let a = dir.path().join("a.wav");
        let b = dir.path().join("b.wav");
        fs::write(&a, b"same bytes").unwrap();
        fs::write(&b, b"same bytes").unwrap();
        let different = dir.path().join("c.wav");
        fs::write(&different, b"different bytes").unwrap();

        let key_a = content_hash_key(&a).unwrap();
        let key_b = content_hash_key(&b).unwrap();
        let key_c = content_hash_key(&different).unwrap();

        assert_eq!(key_a, key_b, "identical content must hash identically");
        assert_ne!(key_a, key_c, "different content must not collide");
    }

    #[test]
    fn atomic_write_is_invisible_until_finalized() {
        let dir = tempfile::TempDir::new().unwrap();
        let final_path = dir.path().join("step").join("out.txt");
        let tmp_path = atomic_temp_path(&final_path);

        // Simulate a crashed writer: tmp exists, final does not.
        fs::create_dir_all(tmp_path.parent().unwrap()).unwrap();
        fs::write(&tmp_path, b"partial or complete, doesn't matter yet").unwrap();
        assert!(!final_path.exists(), "final path must not appear before finalize");

        finalize_atomic(&tmp_path, &final_path).unwrap();
        assert!(final_path.exists());
        assert!(!tmp_path.exists(), "rename must remove the tmp sibling");
        assert_eq!(
            fs::read_to_string(&final_path).unwrap(),
            "partial or complete, doesn't matter yet"
        );
    }

    #[test]
    fn atomic_temp_path_is_a_sibling_with_pid_suffix() {
        let final_path = Path::new("/tmp/voxdecoder-test/meeting.prepared.wav");
        let tmp = atomic_temp_path(final_path);
        assert_eq!(tmp.parent(), final_path.parent());
        assert!(tmp
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("meeting.prepared.wav.tmp-"));
    }
}
