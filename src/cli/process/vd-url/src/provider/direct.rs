//! Direct HTTP(S) media URL.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

use serde_json::json;

use super::tools::find_ffmpeg;
use super::MediaProvider;
use crate::artifact;
use crate::import::{
    prepare_output_dir, ArtifactHandle, ImportError, ImportResult, ProviderId, SubtitlePolicy,
    UrlImportRequest,
};

pub struct DirectProvider;

impl MediaProvider for DirectProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Direct
    }

    fn supports_subtitles(&self) -> bool {
        false
    }

    fn resolve(&self, request: &UrlImportRequest) -> Result<ImportResult, ImportError> {
        if request.subtitles == SubtitlePolicy::Require {
            return Err(ImportError::SubtitlesUnsupported(ProviderId::Direct));
        }
        prepare_output_dir(&request.output_dir, request.overwrite)?;

        let filename = filename_from_url(&request.url);
        let (content_type, content_length) = head_meta(&request.url)?;

        let meta = json!({
            "import": { "provider": "direct" },
            "url": request.url,
            "filename": filename,
            "mime_type": content_type,
            "content_length": content_length,
        });
        let metadata_path = artifact::write_metadata(&request.output_dir, &meta)?;

        if request.metadata_only {
            return Ok(ImportResult {
                provider: ProviderId::Direct,
                audio: None,
                metadata: ArtifactHandle::new("metadata", "metadata", metadata_path),
                subtitle: None,
            });
        }

        let download_path = request.output_dir.join(&filename);
        download_file(&request.url, &download_path)?;

        let audio_path = if looks_like_video(&filename, content_type.as_deref()) {
            let out = request.output_dir.join("audio.wav");
            extract_audio(&download_path, &out)?;
            let _ = fs::remove_file(&download_path);
            out
        } else if is_audio_ext(&filename) {
            let dest = request.output_dir.join(format!(
                "audio.{}",
                Path::new(&filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("bin")
            ));
            if download_path != dest {
                fs::rename(&download_path, &dest).map_err(|e| ImportError::Io(e.to_string()))?;
            }
            dest
        } else {
            // Unknown — try ffmpeg extract; fall back to raw file as audio.
            let out = request.output_dir.join("audio.wav");
            if extract_audio(&download_path, &out).is_ok() {
                let _ = fs::remove_file(&download_path);
                out
            } else {
                let dest = request.output_dir.join("audio.bin");
                fs::rename(&download_path, &dest).map_err(|e| ImportError::Io(e.to_string()))?;
                dest
            }
        };

        Ok(ImportResult {
            provider: ProviderId::Direct,
            audio: Some(ArtifactHandle::new("audio", "audio", audio_path)),
            metadata: ArtifactHandle::new("metadata", "metadata", metadata_path),
            subtitle: None,
        })
    }
}

fn filename_from_url(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    path.rsplit('/')
        .next()
        .filter(|s| !s.is_empty())
        .unwrap_or("download.bin")
        .to_string()
}

fn head_meta(url: &str) -> Result<(Option<String>, Option<u64>), ImportError> {
    let resp = match ureq::head(url).set("User-Agent", "vd-url/0.1").call() {
        Ok(r) => r,
        Err(_) => ureq::get(url)
            .set("User-Agent", "vd-url/0.1")
            .set("Range", "bytes=0-0")
            .call()
            .map_err(|e| ImportError::Provider(format!("HTTP: {e}")))?,
    };
    let ct = resp.header("content-type").map(str::to_string);
    let cl = resp
        .header("content-length")
        .and_then(|s| s.parse::<u64>().ok());
    Ok((ct, cl))
}

fn download_file(url: &str, dest: &Path) -> Result<(), ImportError> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| ImportError::Io(e.to_string()))?;
    }
    let resp = ureq::get(url)
        .set("User-Agent", "vd-url/0.1")
        .call()
        .map_err(|e| ImportError::Provider(format!("HTTP GET: {e}")))?;
    let mut reader = resp.into_reader();
    let mut file = File::create(dest).map_err(|e| ImportError::Io(e.to_string()))?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| ImportError::Io(e.to_string()))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| ImportError::Io(e.to_string()))?;
    }
    Ok(())
}

fn extract_audio(input: &Path, output: &Path) -> Result<(), ImportError> {
    let ffmpeg = find_ffmpeg()?;
    let status = Command::new(ffmpeg)
        .args([
            "-y",
            "-i",
        ])
        .arg(input)
        .args(["-vn", "-acodec", "pcm_s16le", "-ar", "16000", "-ac", "1"])
        .arg(output)
        .status()
        .map_err(|e| ImportError::Provider(format!("ffmpeg: {e}")))?;
    if !status.success() {
        return Err(ImportError::Provider("ffmpeg extract-audio failed".into()));
    }
    Ok(())
}

fn looks_like_video(filename: &str, mime: Option<&str>) -> bool {
    if let Some(m) = mime {
        if m.starts_with("video/") {
            return true;
        }
    }
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v"
    )
}

fn is_audio_ext(filename: &str) -> bool {
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "mp3" | "wav" | "m4a" | "aac" | "ogg" | "opus" | "flac" | "wma"
    )
}
