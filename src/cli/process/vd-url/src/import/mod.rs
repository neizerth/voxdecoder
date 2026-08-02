//! Import request → [`ImportResult`].

mod detect;
mod error;
mod request;
mod result;

pub use detect::{detect_provider, parse_url_ok};
pub use error::ImportError;
pub use request::{SubtitlePolicy, UrlImportRequest};
pub use result::{ArtifactHandle, ImportResult, ProviderId};

use std::path::Path;

use crate::provider::{self, MediaProvider};

/// Resolve a URL into artifacts under `request.output_dir`.
pub fn resolve(request: &UrlImportRequest) -> Result<ImportResult, ImportError> {
    let provider_id = detect_provider(&request.url, request.provider.as_deref())?;
    let provider = provider::resolve_provider(provider_id)?;
    MediaProvider::resolve(provider.as_ref(), request)
}

/// Offline validation: URL shape · provider · subtitle policy support.
pub fn validate_request(
    url: &str,
    provider_hint: Option<&str>,
    subtitles: SubtitlePolicy,
) -> Result<ProviderId, ImportError> {
    if !parse_url_ok(url) {
        return Err(ImportError::InvalidUrl(url.to_string()));
    }
    let id = detect_provider(url, provider_hint)?;
    let provider = provider::resolve_provider(id)?;
    if !provider.supports_subtitles() && subtitles == SubtitlePolicy::Require {
        return Err(ImportError::SubtitlesUnsupported(id));
    }
    Ok(id)
}

/// Ensure output directory exists; error if non-empty conflict without overwrite.
pub fn prepare_output_dir(dir: &Path, overwrite: bool) -> Result<(), ImportError> {
    if dir.exists() {
        if dir.is_file() {
            return Err(ImportError::Io(format!(
                "output-dir is a file: {}",
                dir.display()
            )));
        }
        if !overwrite {
            let has_entries = std::fs::read_dir(dir)
                .map_err(|e| ImportError::Io(e.to_string()))?
                .next()
                .is_some();
            if has_entries {
                // Allow reuse of same dir when overwrite; otherwise warn only if known artifacts exist.
                for name in ["audio.m4a", "audio.wav", "metadata.yaml", "subtitles.vtt"] {
                    let p = dir.join(name);
                    if p.exists() {
                        return Err(ImportError::Exists(p));
                    }
                }
            }
        }
    } else {
        std::fs::create_dir_all(dir).map_err(|e| ImportError::Io(e.to_string()))?;
    }
    Ok(())
}
