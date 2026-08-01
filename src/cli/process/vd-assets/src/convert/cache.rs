//! On-disk cache for extracted document text / dictionaries.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::OcrMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMeta {
    pub version: u32,
    pub source: String,
    pub source_len: u64,
    pub source_mtime_secs: u64,
    pub ocr: bool,
    pub created_unix: u64,
}

#[derive(Debug, Clone)]
pub struct CacheHit {
    pub meta_path: PathBuf,
    pub text_path: PathBuf,
    pub dict_path: PathBuf,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CacheStore {
    root: PathBuf,
}

impl CacheStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_store() -> Self {
        Self::new(crate::paths::cache_root())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Look up a valid cache entry for `source` + `ocr`.
    pub fn get(&self, source: &Path, ocr: OcrMode) -> Option<CacheHit> {
        let key = cache_key(source, ocr);
        let meta_path = self.root.join(format!("{key}.meta.json"));
        let text_path = self.root.join(format!("{key}.txt"));
        let dict_path = self.root.join(format!("{key}.dict.yaml"));
        if !meta_path.exists() || !text_path.exists() {
            return None;
        }
        let meta_raw = fs::read_to_string(&meta_path).ok()?;
        let meta: CacheMeta = serde_json::from_str(&meta_raw).ok()?;
        let (len, mtime) = source_fingerprint(source).ok()?;
        if meta.version != 1
            || meta.source_len != len
            || meta.source_mtime_secs != mtime
            || meta.ocr != ocr.enabled()
        {
            return None;
        }
        let text = fs::read_to_string(&text_path).ok()?;
        Some(CacheHit {
            meta_path,
            text_path,
            dict_path,
            text,
        })
    }

    /// Persist extracted text (+ optional dict yaml body).
    pub fn put(
        &self,
        source: &Path,
        ocr: OcrMode,
        text: &str,
        dict_yaml: Option<&str>,
    ) -> Result<CacheHit, String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let key = cache_key(source, ocr);
        let meta_path = self.root.join(format!("{key}.meta.json"));
        let text_path = self.root.join(format!("{key}.txt"));
        let dict_path = self.root.join(format!("{key}.dict.yaml"));
        let (len, mtime) = source_fingerprint(source)?;
        let meta = CacheMeta {
            version: 1,
            source: source.display().to_string(),
            source_len: len,
            source_mtime_secs: mtime,
            ocr: ocr.enabled(),
            created_unix: unix_now(),
        };
        fs::write(
            &meta_path,
            serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::write(&text_path, text).map_err(|e| e.to_string())?;
        if let Some(body) = dict_yaml {
            fs::write(&dict_path, body).map_err(|e| e.to_string())?;
        } else if !dict_path.exists() {
            // still record empty dict path for callers
            fs::write(&dict_path, "").map_err(|e| e.to_string())?;
        }
        Ok(CacheHit {
            meta_path,
            text_path,
            dict_path,
            text: text.to_string(),
        })
    }
}

fn cache_key(source: &Path, ocr: OcrMode) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.display().to_string().as_bytes());
    hasher.update([u8::from(ocr.enabled())]);
    let digest = hasher.finalize();
    let mut key = String::with_capacity(32);
    for b in digest.iter().take(16) {
        use std::fmt::Write;
        let _ = write!(key, "{b:02x}");
    }
    key
}

fn source_fingerprint(path: &Path) -> Result<(u64, u64), String> {
    let meta = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = meta.len();
    let mtime = meta
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok((len, mtime))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
