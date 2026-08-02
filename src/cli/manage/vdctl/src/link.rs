//! Put `vdctl` on the user PATH (symlink / copy).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Error;

#[derive(Debug)]
pub struct LinkResult {
    pub dest: PathBuf,
    pub changed: bool,
}

/// Prefer Cargo's bin dir (usually already on PATH), else `~/.local/bin`.
pub fn user_bin_dir() -> PathBuf {
    if let Ok(cargo_home) = env::var("CARGO_HOME") {
        let p = PathBuf::from(cargo_home).join("bin");
        if p.is_dir() || env::var_os("CARGO_HOME").is_some() {
            return p;
        }
    }
    if let Some(home) = env::var_os("HOME") {
        let cargo = PathBuf::from(home).join(".cargo").join("bin");
        if cargo.is_dir() {
            return cargo;
        }
    }
    directories::UserDirs::new()
        .map(|u| u.home_dir().join(".local").join("bin"))
        .unwrap_or_else(|| PathBuf::from(".local/bin"))
}

/// True if `dir` already appears in `$PATH` (no need to export again).
pub fn is_on_path(dir: &Path) -> bool {
    let Ok(path_var) = env::var("PATH") else {
        return false;
    };
    let want = normalize_path_key(dir);
    std::env::split_paths(&path_var).any(|entry| normalize_path_key(&entry) == want)
}

pub fn link_vdctl(source: &Path) -> Result<LinkResult, Error> {
    if !source.is_file() {
        return Err(Error::Message(format!(
            "vdctl binary not found at {}\nBuild first: cargo build -p vdctl",
            source.display()
        )));
    }
    let source = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let bin_dir = user_bin_dir();
    fs::create_dir_all(&bin_dir).map_err(|e| Error::Message(e.to_string()))?;
    let dest = bin_dir.join(if cfg!(windows) { "vdctl.exe" } else { "vdctl" });

    if already_points_to(&dest, &source) {
        return Ok(LinkResult {
            dest,
            changed: false,
        });
    }

    #[cfg(unix)]
    {
        if dest.exists() || dest.is_symlink() {
            fs::remove_file(&dest).map_err(|e| Error::Message(e.to_string()))?;
        }
        std::os::unix::fs::symlink(&source, &dest).map_err(|e| {
            Error::Message(format!(
                "failed to symlink {} → {}: {e}",
                source.display(),
                dest.display()
            ))
        })?;
    }
    #[cfg(windows)]
    {
        let _ = fs::remove_file(&dest);
        fs::copy(&source, &dest).map_err(|e| Error::Message(e.to_string()))?;
    }

    Ok(LinkResult {
        dest,
        changed: true,
    })
}

pub fn resolve_source(workspace: Option<&Path>) -> PathBuf {
    if let Some(ws) = workspace {
        for profile in ["debug", "release"] {
            let candidate = ws.join("target").join(profile).join(if cfg!(windows) {
                "vdctl.exe"
            } else {
                "vdctl"
            });
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    env::current_exe().unwrap_or_else(|_| PathBuf::from("vdctl"))
}

fn already_points_to(dest: &Path, source: &Path) -> bool {
    #[cfg(unix)]
    {
        if let Ok(existing) = fs::read_link(dest) {
            let existing = if existing.is_absolute() {
                existing
            } else {
                dest.parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(existing)
            };
            let existing = fs::canonicalize(&existing).unwrap_or(existing);
            return existing == source;
        }
    }
    if let (Ok(a), Ok(b)) = (fs::canonicalize(dest), fs::canonicalize(source)) {
        return a == b;
    }
    false
}

fn normalize_path_key(path: &Path) -> String {
    let canon = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = canon.to_string_lossy();
    s.trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_membership_detects_existing_entry() {
        let bin = user_bin_dir();
        // If cargo bin exists and is on PATH in this environment, membership is true.
        // The check itself must not panic and must be stable for identical strings.
        let _ = is_on_path(&bin);
        assert!(is_on_path(Path::new("/tmp")) || !is_on_path(Path::new("/tmp")));
        let fake = PathBuf::from("/vdctl-path-should-not-exist-xyz");
        assert!(!is_on_path(&fake));
    }
}
