use vd_url::{detect_provider, parse_url_ok, ProviderId};

#[test]
fn rejects_bad_url() {
    assert!(!parse_url_ok("ftp://x"));
    assert!(!parse_url_ok("not-a-url"));
    assert!(parse_url_ok("https://youtu.be/abcdefghijk"));
}

#[test]
fn detects_youtube() {
    assert_eq!(
        detect_provider("https://www.youtube.com/watch?v=abcdefghijk", None).unwrap(),
        ProviderId::Youtube
    );
}

#[test]
fn hint_stub() {
    assert_eq!(
        detect_provider("https://example.com/a.mp3", Some("stub")).unwrap(),
        ProviderId::Stub
    );
}
