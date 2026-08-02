use vd_url::{validate_request, ProviderId, SubtitlePolicy};

#[test]
fn require_on_direct_fails() {
    let err = validate_request(
        "https://cdn.example.com/a.mp3",
        Some("direct"),
        SubtitlePolicy::Require,
    );
    assert!(err.is_err());
}

#[test]
fn prefer_on_youtube_ok() {
    let id = validate_request(
        "https://youtu.be/abcdefghijk",
        None,
        SubtitlePolicy::Prefer,
    )
    .unwrap();
    assert_eq!(id, ProviderId::Youtube);
}

#[test]
fn stub_supports_require() {
    let id = validate_request(
        "https://example.com/x",
        Some("stub"),
        SubtitlePolicy::Require,
    )
    .unwrap();
    assert_eq!(id, ProviderId::Stub);
}
