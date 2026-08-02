//! YouTube via yt-dlp.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

use super::tools::find_ytdlp;
use super::MediaProvider;
use crate::artifact;
use crate::import::{
    prepare_output_dir, ArtifactHandle, ImportError, ImportResult, ProviderId, SubtitlePolicy,
    UrlImportRequest,
};

pub struct YoutubeProvider;

impl MediaProvider for YoutubeProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Youtube
    }

    fn supports_subtitles(&self) -> bool {
        true
    }

    fn resolve(&self, request: &UrlImportRequest) -> Result<ImportResult, ImportError> {
        prepare_output_dir(&request.output_dir, request.overwrite)?;
        let ytdlp = find_ytdlp()?;
        let info = dump_json(&ytdlp, &request.url)?;

        let subs_available = subtitles_available(&info);
        match request.subtitles {
            SubtitlePolicy::Require if !subs_available => {
                return Err(ImportError::SubtitlesRequired);
            }
            _ => {}
        }

        let metadata_path = write_youtube_metadata(&request.output_dir, &request.url, &info)?;

        if request.metadata_only {
            return Ok(ImportResult {
                provider: ProviderId::Youtube,
                audio: None,
                metadata: ArtifactHandle::new("metadata", "metadata", metadata_path),
                subtitle: None,
            });
        }

        let audio_path = download_audio(&ytdlp, &request.url, &request.output_dir)?;
        let subtitle = match request.subtitles {
            SubtitlePolicy::Ignore => None,
            SubtitlePolicy::Prefer if !subs_available => None,
            SubtitlePolicy::Prefer | SubtitlePolicy::Require => {
                download_subtitles(&ytdlp, &request.url, &request.output_dir)?
            }
        };

        Ok(ImportResult {
            provider: ProviderId::Youtube,
            audio: Some(ArtifactHandle::new("audio", "audio", audio_path)),
            metadata: ArtifactHandle::new("metadata", "metadata", metadata_path),
            subtitle,
        })
    }
}

fn dump_json(ytdlp: &Path, url: &str) -> Result<Value, ImportError> {
    let out = Command::new(ytdlp)
        .args(["--dump-json", "--no-download", "--no-warnings", url])
        .output()
        .map_err(|e| ImportError::Provider(format!("yt-dlp: {e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(ImportError::Provider(format!(
            "yt-dlp dump-json failed: {err}"
        )));
    }
    serde_json::from_slice(&out.stdout)
        .map_err(|e| ImportError::Provider(format!("yt-dlp JSON: {e}")))
}

fn subtitles_available(info: &Value) -> bool {
    let has = |key: &str| {
        info.get(key)
            .and_then(Value::as_object)
            .is_some_and(|m| !m.is_empty())
    };
    has("subtitles") || has("automatic_captions")
}

fn write_youtube_metadata(dir: &Path, url: &str, info: &Value) -> Result<PathBuf, ImportError> {
    let chapters = info.get("chapters").cloned().unwrap_or(json!([]));
    let subs = info.get("subtitles").cloned().unwrap_or(json!({}));
    let auto = info
        .get("automatic_captions")
        .cloned()
        .unwrap_or(json!({}));
    let meta = json!({
        "import": { "provider": "youtube" },
        "url": url,
        "video_id": info.get("id"),
        "title": info.get("title"),
        "channel": info.get("channel").or_else(|| info.get("uploader")),
        "published_at": info.get("upload_date"),
        "duration": info.get("duration"),
        "language": info.get("language"),
        "chapters": chapters,
        "thumbnail": info.get("thumbnail"),
        "subtitles_available": subtitles_available(info),
        "subtitle_languages": {
            "manual": subs.as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "automatic": auto.as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default(),
        },
    });
    artifact::write_metadata(dir, &meta)
}

fn download_audio(ytdlp: &Path, url: &str, dir: &Path) -> Result<PathBuf, ImportError> {
    let template = dir.join("audio.%(ext)s");
    let status = Command::new(ytdlp)
        .args([
            "-x",
            "--audio-format",
            "m4a",
            "--audio-quality",
            "0",
            "--no-playlist",
            "--no-warnings",
            "-o",
        ])
        .arg(&template)
        .arg(url)
        .status()
        .map_err(|e| ImportError::Provider(format!("yt-dlp audio: {e}")))?;
    if !status.success() {
        return Err(ImportError::Provider("yt-dlp audio download failed".into()));
    }
    find_audio_file(dir)
}

fn download_subtitles(
    ytdlp: &Path,
    url: &str,
    dir: &Path,
) -> Result<Option<ArtifactHandle>, ImportError> {
    let template = dir.join("subs");
    let status = Command::new(ytdlp)
        .args([
            "--skip-download",
            "--write-subs",
            "--write-auto-subs",
            "--sub-format",
            "vtt/best",
            "--convert-subs",
            "vtt",
            "--no-playlist",
            "--no-warnings",
            "-o",
        ])
        .arg(&template)
        .arg(url)
        .status()
        .map_err(|e| ImportError::Provider(format!("yt-dlp subs: {e}")))?;
    if !status.success() {
        return Ok(None);
    }
    let found = find_subtitle_file(dir)?;
    Ok(found.map(|p| ArtifactHandle::new("subtitle", "subtitle", p)))
}

fn find_audio_file(dir: &Path) -> Result<PathBuf, ImportError> {
    for name in ["audio.m4a", "audio.webm", "audio.opus", "audio.mp3", "audio.wav"] {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
    }
    // yt-dlp may use id-based names if template ignored — pick newest audio-like file.
    let mut best: Option<PathBuf> = None;
    if let Ok(rd) = fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(ext.as_str(), "m4a" | "webm" | "opus" | "mp3" | "wav" | "ogg") {
                best = Some(p);
            }
        }
    }
    best.ok_or_else(|| ImportError::Provider("audio file not found after yt-dlp".into()))
}

fn find_subtitle_file(dir: &Path) -> Result<Option<PathBuf>, ImportError> {
    let target = dir.join("subtitles.vtt");
    if let Ok(rd) = fs::read_dir(dir) {
        for ent in rd.flatten() {
            let p = ent.path();
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".vtt") || name.ends_with(".srt") {
                if p != target {
                    let _ = fs::rename(&p, &target);
                }
                return Ok(Some(if target.exists() { target } else { p }));
            }
        }
    }
    Ok(None)
}
