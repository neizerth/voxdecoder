//! Provider detection from URL + hint.

use super::{ImportError, ProviderId};

/// Basic URL shape check (no network).
pub fn parse_url_ok(url: &str) -> bool {
    let u = url.trim();
    (u.starts_with("http://") || u.starts_with("https://")) && u.len() > 10
}

/// Resolve provider: explicit hint (unless `auto`) → host heuristics → `direct`.
pub fn detect_provider(url: &str, hint: Option<&str>) -> Result<ProviderId, ImportError> {
    if !parse_url_ok(url) {
        return Err(ImportError::InvalidUrl(url.to_string()));
    }
    if let Some(h) = hint {
        let h = h.trim();
        if !h.is_empty() && !h.eq_ignore_ascii_case("auto") {
            return ProviderId::parse(h).ok_or_else(|| ImportError::UnknownProvider(h.to_string()));
        }
    }
    Ok(detect_from_url(url))
}

fn detect_from_url(url: &str) -> ProviderId {
    let lower = url.to_ascii_lowercase();
    if lower.contains("youtube.com")
        || lower.contains("youtu.be")
        || lower.contains("youtube-nocookie.com")
        || lower.contains("music.youtube.com")
    {
        return ProviderId::Youtube;
    }
    ProviderId::Direct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_hosts() {
        assert_eq!(
            detect_provider("https://youtu.be/abcdefghijk", None).unwrap(),
            ProviderId::Youtube
        );
        assert_eq!(
            detect_provider("https://www.youtube.com/watch?v=abcdefghijk", None).unwrap(),
            ProviderId::Youtube
        );
    }

    #[test]
    fn direct_fallback() {
        assert_eq!(
            detect_provider("https://cdn.example.com/a.mp3", None).unwrap(),
            ProviderId::Direct
        );
    }

    #[test]
    fn hint_overrides() {
        assert_eq!(
            detect_provider("https://youtu.be/abcdefghijk", Some("direct")).unwrap(),
            ProviderId::Direct
        );
        assert_eq!(
            detect_provider("https://cdn.example.com/a.mp3", Some("stub")).unwrap(),
            ProviderId::Stub
        );
    }
}
