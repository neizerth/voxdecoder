//! ExecutionReport unit tests.

use std::collections::BTreeMap;
use std::time::{Duration, UNIX_EPOCH};

use vd_pipeline::{
    backend_from_options, format_rfc3339, model_from_options, ArgValue, ArtifactStat,
    ExecutionReport, JobReportStatus, StepReport, StepReportStatus,
};

// Re-export helpers used via crate internals — tests use public report types.
// backend_from_options / format_rfc3339 need to be pub use from lib.

#[test]
fn rfc3339_epoch() {
    let t = UNIX_EPOCH + Duration::from_secs(0);
    assert_eq!(format_rfc3339(t), "1970-01-01T00:00:00.000Z");
}

#[test]
fn rfc3339_known() {
    let t = UNIX_EPOCH + Duration::from_secs(1_785_664_800);
    assert_eq!(format_rfc3339(t), "2026-08-02T10:00:00.000Z");
}

#[test]
fn backend_prefers_engine() {
    let mut opts = BTreeMap::new();
    opts.insert("engine".into(), ArgValue::String("gigaam".into()));
    opts.insert("provider".into(), ArgValue::String("openai".into()));
    assert_eq!(backend_from_options(&opts).as_deref(), Some("gigaam"));
}

#[test]
fn backend_nested_provider() {
    let mut nested = BTreeMap::new();
    nested.insert("provider".into(), ArgValue::String("stub".into()));
    nested.insert("model".into(), ArgValue::String("det-v1".into()));
    let mut opts = BTreeMap::new();
    opts.insert("backend".into(), ArgValue::Map(nested));
    assert_eq!(backend_from_options(&opts).as_deref(), Some("stub"));
    assert_eq!(model_from_options(&opts).as_deref(), Some("det-v1"));
}

#[test]
fn report_json_shape() {
    let report = ExecutionReport {
        version: 1,
        job: Some("meeting".into()),
        status: JobReportStatus::Ok,
        started_at: "2026-08-02T10:00:00.000Z".into(),
        finished_at: "2026-08-02T10:01:12.000Z".into(),
        duration_ms: 72_000,
        critical_path_ms: Some(48_000),
        parallel_efficiency: Some(0.9),
        steps: vec![StepReport {
            id: "transcribe".into(),
            capability: "transcribe".into(),
            name: None,
            status: StepReportStatus::Ok,
            queued_at: "2026-08-02T10:00:00.000Z".into(),
            started_at: "2026-08-02T10:00:00.000Z".into(),
            finished_at: "2026-08-02T10:00:48.000Z".into(),
            duration_ms: 48_000,
            backend: Some("gigaam".into()),
            model: Some("v3_e2e_ctc".into()),
            phases: vec![],
            inputs: vec![ArtifactStat {
                path: "meeting.wav".into(),
                bytes: Some(1024),
            }],
            outputs: vec![ArtifactStat {
                path: "meeting.txt".into(),
                bytes: Some(200),
            }],
        }],
    };
    let json = report.to_json_pretty().unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["status"], "ok");
    assert_eq!(v["steps"][0]["duration_ms"], 48_000);
    assert_eq!(v["steps"][0]["backend"], "gigaam");
    assert!(
        v["steps"][0].get("phases").is_none()
            || v["steps"][0]["phases"].as_array().unwrap().is_empty()
    );
}

#[test]
fn skipped_status_serializes() {
    let s = serde_json::to_value(StepReportStatus::Skipped).unwrap();
    assert_eq!(s, "skipped");
}
