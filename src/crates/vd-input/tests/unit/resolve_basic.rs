use std::path::Path;

use tempfile::tempdir;
use vd_input::{resolve, InputSource, ResolveContext, SourceKind, SubtitlePolicy};

#[test]
fn file_passthrough() {
    let dir = tempdir().unwrap();
    let src = InputSource {
        path: Some("/tmp/a.wav".into()),
        ..Default::default()
    };
    let ctx = ResolveContext::new(dir.path());
    let r = resolve(&src, &ctx, None).unwrap();
    assert_eq!(r.kind, SourceKind::File);
    assert_eq!(r.audio.as_deref(), Some(Path::new("/tmp/a.wav")));
}

#[test]
fn url_stub_materializes_audio() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("imp");
    let src = InputSource {
        url: Some("https://example.com/x".into()),
        ..Default::default()
    };
    let mut ctx = ResolveContext::new(dir.path());
    ctx.output_dir = Some(&out);
    ctx.provider_hint = Some("stub");
    ctx.subtitles = SubtitlePolicy::Prefer;
    let r = resolve(&src, &ctx, None).unwrap();
    assert_eq!(r.kind, SourceKind::Url);
    assert!(r.audio.as_ref().unwrap().is_file());
    assert!(r.metadata.as_ref().unwrap().is_file());
    assert!(r.subtitle.as_ref().unwrap().is_file());
}

#[test]
fn xor_enforced() {
    let dir = tempdir().unwrap();
    let src = InputSource {
        path: Some("/a".into()),
        url: Some("https://x".into()),
        ..Default::default()
    };
    assert!(resolve(&src, &ResolveContext::new(dir.path()), None).is_err());
}
