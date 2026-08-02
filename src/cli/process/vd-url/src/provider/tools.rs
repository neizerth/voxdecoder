//! Locate yt-dlp / ffmpeg (`VD_YTDLP` · `VD_FFMPEG`).

use std::path::PathBuf;
use std::process::Command;

use crate::import::ImportError;

pub struct DoctorCheck {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
}

pub fn doctor_report() -> Vec<DoctorCheck> {
    vec![
        check_tool("yt-dlp", find_ytdlp()),
        check_tool("ffmpeg", find_ffmpeg()),
    ]
}

fn check_tool(name: &'static str, found: Result<PathBuf, ImportError>) -> DoctorCheck {
    match found {
        Ok(path) => {
            let ver = version_line(&path).unwrap_or_default();
            let detail = if ver.is_empty() {
                path.display().to_string()
            } else {
                format!("{}  ({ver})", path.display())
            };
            DoctorCheck {
                name,
                ok: true,
                detail,
            }
        }
        Err(e) => DoctorCheck {
            name,
            ok: false,
            detail: e.to_string(),
        },
    }
}

fn version_line(bin: &PathBuf) -> Option<String> {
    let out = Command::new(bin).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().next().map(str::trim).map(str::to_string)
}

pub fn find_ytdlp() -> Result<PathBuf, ImportError> {
    if let Ok(p) = std::env::var("VD_YTDLP") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }
    which("yt-dlp").ok_or_else(|| {
        ImportError::Unavailable("yt-dlp not found on PATH (set VD_YTDLP)".into())
    })
}

pub fn find_ffmpeg() -> Result<PathBuf, ImportError> {
    if let Ok(p) = std::env::var("VD_FFMPEG") {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(path);
        }
    }
    which("ffmpeg")
        .ok_or_else(|| ImportError::Unavailable("ffmpeg not found on PATH (set VD_FFMPEG)".into()))
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let candidate = dir.join(bin);
            if candidate.is_file() {
                return Some(candidate);
            }
            #[cfg(windows)]
            {
                let exe = dir.join(format!("{bin}.exe"));
                if exe.is_file() {
                    return Some(exe);
                }
            }
        }
        None
    })
}
