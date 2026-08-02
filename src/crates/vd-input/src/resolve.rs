//! Resolve InputSource → ResolvedInput.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use vd_url::{resolve as resolve_url, SubtitlePolicy, UrlImportRequest};

use crate::error::InputError;
use crate::resolved::{ResolvedInput, SourceKind};
use crate::source::InputSource;

/// Context for materializing inputs on the Runtime host.
pub struct ResolveContext<'a> {
    pub data_dir: &'a Path,
    /// Directory for URL / blob materialization (defaults under `data_dir/imports`).
    pub output_dir: Option<&'a Path>,
    pub subtitles: SubtitlePolicy,
    pub provider_hint: Option<&'a str>,
    pub overwrite: bool,
    pub metadata_only: bool,
}

impl<'a> ResolveContext<'a> {
    pub fn new(data_dir: &'a Path) -> Self {
        Self {
            data_dir,
            output_dir: None,
            subtitles: SubtitlePolicy::Ignore,
            provider_hint: None,
            overwrite: true,
            metadata_only: false,
        }
    }
}

/// Resolve a user [`InputSource`] into Runtime artifacts.
pub fn resolve(
    source: &InputSource,
    ctx: &ResolveContext<'_>,
    artifact_lookup: Option<&dyn Fn(&str) -> Result<PathBuf, String>>,
) -> Result<ResolvedInput, InputError> {
    source.validate_xor()?;

    if let Some(path) = &source.path {
        return resolve_file(path);
    }
    if let Some(uri) = &source.uri {
        return resolve_uri(uri);
    }
    if let Some(url) = source.as_url() {
        return resolve_url_source(url, ctx);
    }
    if let Some(id) = &source.artifact {
        return resolve_artifact(id, artifact_lookup);
    }
    if let Some(blob) = &source.blob {
        return resolve_blob(blob, ctx);
    }
    Err(InputError::Invalid("empty InputSource".into()))
}

fn resolve_file(path: &Path) -> Result<ResolvedInput, InputError> {
    Ok(ResolvedInput {
        kind: SourceKind::File,
        audio: Some(path.to_path_buf()),
        metadata: None,
        subtitle: None,
        provider: None,
    })
}

fn resolve_uri(uri: &str) -> Result<ResolvedInput, InputError> {
    let path = uri
        .strip_prefix("file://")
        .map(PathBuf::from)
        .ok_or_else(|| InputError::Invalid(format!("unsupported input URI scheme: {uri}")))?;
    resolve_file(&path)
}

fn resolve_url_source(url: &str, ctx: &ResolveContext<'_>) -> Result<ResolvedInput, InputError> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(InputError::Invalid(format!("url must be http(s): {url}")));
    }
    let out = ctx.output_dir.map_or_else(
        || {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            ctx.data_dir.join("imports").join(format!("url-{nonce}"))
        },
        Path::to_path_buf,
    );
    fs::create_dir_all(&out).map_err(|e| InputError::Io(e.to_string()))?;

    let result = resolve_url(&UrlImportRequest {
        url: url.to_string(),
        provider: ctx.provider_hint.map(str::to_string),
        subtitles: ctx.subtitles,
        metadata_only: ctx.metadata_only,
        output_dir: out,
        overwrite: ctx.overwrite,
    })
    .map_err(|e| InputError::Provider(e.to_string()))?;

    Ok(ResolvedInput {
        kind: SourceKind::Url,
        audio: result.audio.map(|a| a.path),
        metadata: Some(result.metadata.path),
        subtitle: result.subtitle.map(|s| s.path),
        provider: Some(result.provider.as_str().to_string()),
    })
}

fn resolve_artifact(
    id: &str,
    artifact_lookup: Option<&dyn Fn(&str) -> Result<PathBuf, String>>,
) -> Result<ResolvedInput, InputError> {
    let lookup = artifact_lookup.ok_or_else(|| {
        InputError::Invalid("artifact inputs require a Runtime Job Store".into())
    })?;
    let path = lookup(id).map_err(InputError::Invalid)?;
    Ok(ResolvedInput {
        kind: SourceKind::Artifact,
        audio: Some(path),
        metadata: None,
        subtitle: None,
        provider: None,
    })
}

fn resolve_blob(blob: &str, ctx: &ResolveContext<'_>) -> Result<ResolvedInput, InputError> {
    let dir = ctx.data_dir.join("inputs");
    fs::create_dir_all(&dir).map_err(|e| InputError::Io(e.to_string()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = dir.join(format!("blob-{nonce}.bin"));
    fs::write(&path, blob.as_bytes()).map_err(|e| InputError::Io(e.to_string()))?;
    Ok(ResolvedInput {
        kind: SourceKind::Blob,
        audio: Some(path),
        metadata: None,
        subtitle: None,
        provider: None,
    })
}
