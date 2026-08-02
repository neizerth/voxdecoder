//! Filter sugar / parse.

use vd_preprocess::preprocess::{expand_and_validate, parse_filter_flag, RawFilter};

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
