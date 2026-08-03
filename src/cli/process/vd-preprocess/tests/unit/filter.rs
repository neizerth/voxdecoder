//! Filter sugar / parse.

use std::path::Path;

use vd_preprocess::preprocess::{
    expand_and_validate, ffmpeg_argv_for_plan, parse_filter_flag, FilterSpec, RawFilter,
};

#[test]
fn type_sugar_expands() {
    let raw = vec![RawFilter {
        provider: None,
        operation: None,
        r#type: Some("normalize".into()),
        params: Default::default(),
    }];
    let specs = expand_and_validate(raw, "ffmpeg").unwrap();
    assert_eq!(specs[0].provider, "ffmpeg");
    assert_eq!(specs[0].operation, "normalize");
}

#[test]
fn empty_chain_errors() {
    let err = expand_and_validate(vec![], "stub").unwrap_err();
    assert_eq!(err.exit_code(), 2);
}

#[test]
fn parse_filter_with_params() {
    let f = parse_filter_flag("speed:factor=1.15", "stub").unwrap();
    assert_eq!(f.operation, "speed");
    assert_eq!(f.params.get("factor").and_then(|v| v.as_f64()), Some(1.15));
}

#[test]
fn pad_start_ffmpeg_args() {
    let f = parse_filter_flag("pad-start:duration_sec=2.5", "ffmpeg").unwrap();
    let args = ffmpeg_argv_for_plan(&f, Path::new("in.wav"), Path::new("out.wav")).unwrap();
    assert!(args.iter().any(|a| a == "adelay=delays=2500:all=1"));
}

#[test]
fn pad_end_ffmpeg_args() {
    let f = parse_filter_flag("pad-end:duration_sec=1.25", "ffmpeg").unwrap();
    let args = ffmpeg_argv_for_plan(&f, Path::new("in.wav"), Path::new("out.wav")).unwrap();
    assert!(args.iter().any(|a| a == "apad=pad_dur=1.25"));
}

#[test]
fn pad_start_known_in_catalog() {
    let raw = vec![RawFilter {
        provider: None,
        operation: None,
        r#type: Some("pad-start".into()),
        params: [(
            "duration_sec".into(),
            serde_json::json!(3.0),
        )]
        .into_iter()
        .collect(),
    }];
    let specs = expand_and_validate(raw, "ffmpeg").unwrap();
    assert_eq!(specs[0].operation, "pad-start");
    let _ = FilterSpec {
        provider: specs[0].provider.clone(),
        operation: specs[0].operation.clone(),
        params: specs[0].params.clone(),
    };
}
