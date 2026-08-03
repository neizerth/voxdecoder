//! `vd-fix-overlap run` implementation.
//!
//! Reads a real diarized JSON/JSONL artifact via `vd_artifact::collect_segments`
//! (speaker + start_sec + end_sec + text together), runs `overlap::detect_duplicates`,
//! and reports candidate pairs. With `--apply` (or any output flag), removes the
//! `drop` side of every pair via `vd_artifact::remove_segments` and writes a fixed
//! artifact — see `docs/adr/0010-...` for why this narrow structural primitive
//! exists only for JSON/JSONL turn arrays.

use std::path::PathBuf;

use serde::Serialize;
use vd_artifact::{Segment, SegmentId};
use vd_output::{OutputPathRequest, OutputPaths};

use super::CliError;
use crate::config;
use crate::overlap::{detect_duplicates, DuplicateKind, DuplicatePair, TrimAction, Utterance};

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub overwrite: bool,
    pub apply: bool,
    pub similarity_threshold: Option<f64>,
    pub max_gap_ms: Option<u64>,
    pub json: bool,
    pub quiet: bool,
}

#[derive(Debug, Serialize)]
struct ReportPair {
    keep: usize,
    keep_speaker: String,
    drop: usize,
    drop_speaker: String,
    kind: &'static str,
    similarity: f64,
    /// What `--apply` would do (or did) to `drop`: `"remove"` the whole
    /// turn, or `"trim"` it down to `trimmed_text` (ADR 0012 §2 partial
    /// duplicates — `drop` contains `keep` plus a unique remainder).
    action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    trimmed_text: Option<String>,
}

pub fn execute(args: RunArgs) -> Result<(), CliError> {
    if !args.input.exists() {
        return Err(CliError::with_code(
            3,
            format!("input file missing / unreadable: {}", args.input.display()),
        ));
    }

    let mut artifact = vd_artifact::load(&args.input)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;

    let segments = vd_artifact::collect_segments(&artifact);
    if segments.is_empty() {
        return Err(CliError::with_code(
            3,
            "no speaker turns found (need JSON/JSONL objects with speaker + start_sec/end_sec + text — Txt/Md/Srt/Vtt carry no structural speaker field)",
        ));
    }
    let utterances: Vec<Utterance> = segments.iter().map(segment_to_utterance).collect();

    let file = config::load(&crate::paths::config_path()).map_err(CliError::usage)?;
    let opts = file.resolve(args.similarity_threshold, args.max_gap_ms);

    let pairs = detect_duplicates(&utterances, &opts);

    if args.json {
        let report: Vec<ReportPair> = pairs
            .iter()
            .map(|p| to_report_pair(p, &utterances))
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|e| CliError::with_code(1, e.to_string()))?
        );
    } else {
        print_text_report(&pairs, &utterances, args.quiet);
    }

    if !args.apply || pairs.is_empty() {
        return Ok(());
    }

    let mut remove_ids: Vec<SegmentId> = Vec::new();
    let mut trim_ops: Vec<(SegmentId, &str)> = Vec::new();
    for pair in &pairs {
        let id = segments[pair.drop].id;
        match &pair.trim {
            TrimAction::RemoveWhole => remove_ids.push(id),
            TrimAction::TrimTo(text) => trim_ops.push((id, text.as_str())),
        }
    }
    let removed = vd_artifact::remove_segments(&mut artifact, &remove_ids);
    let mut trimmed = 0usize;
    for (id, text) in &trim_ops {
        if vd_artifact::set_segment_text(&mut artifact, *id, text) {
            trimmed += 1;
        }
    }

    let paths = resolve_output(&args, &artifact)?;
    vd_artifact::write(&artifact, &paths.main)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;

    if !args.quiet && !args.json {
        println!(
            "Removed {removed} duplicate turn(s), trimmed {trimmed} -> {}",
            paths.main.display()
        );
    }

    Ok(())
}

fn resolve_output(
    args: &RunArgs,
    artifact: &vd_artifact::Artifact,
) -> Result<OutputPaths, CliError> {
    let default_file_name =
        vd_output::fixed_file_name(&args.input, artifact.artifact_type().extension());
    vd_output::resolve_output_path(OutputPathRequest {
        input: args.input.clone(),
        output: args.output.clone(),
        output_dir: args.output_dir.clone(),
        in_place: args.in_place,
        overwrite: args.overwrite || args.in_place,
        default_file_name,
    })
    .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))
}

fn print_text_report(pairs: &[DuplicatePair], utterances: &[Utterance], quiet: bool) {
    if pairs.is_empty() {
        if !quiet {
            println!(
                "No duplicate speech detected ({} turns checked).",
                utterances.len()
            );
        }
        return;
    }
    if !quiet {
        println!(
            "{} candidate duplicate pair(s) found ({} turns checked):",
            pairs.len(),
            utterances.len()
        );
    }
    for pair in pairs {
        let report = to_report_pair(pair, utterances);
        let action = report
            .trimmed_text
            .as_ref()
            .map_or_else(|| "remove".to_string(), |text| format!("trim to {text:?}"));
        println!(
            "  [{}] keep={} ({}) drop={} ({}) similarity={:.2} -> {action}",
            report.kind,
            report.keep,
            report.keep_speaker,
            report.drop,
            report.drop_speaker,
            report.similarity,
        );
    }
}

/// `start_sec`/`end_sec` (seconds, `f64`) → `start_ms`/`end_ms` (`u64`) for
/// the detection algorithm, which works in whole milliseconds. Missing
/// timing collapses to `0`, which only matters for the (rare) input that
/// omits timestamps entirely — such turns will simply always look
/// "temporally close" to each other.
fn segment_to_utterance(seg: &Segment) -> Utterance {
    Utterance {
        speaker: seg.speaker.clone().unwrap_or_default(),
        text: seg.text.clone(),
        start_ms: seg.start_sec.map_or(0, |s| (s * 1000.0).round() as u64),
        end_ms: seg.end_sec.map_or(0, |s| (s * 1000.0).round() as u64),
    }
}

fn to_report_pair(pair: &DuplicatePair, utterances: &[Utterance]) -> ReportPair {
    let (action, trimmed_text) = match &pair.trim {
        TrimAction::RemoveWhole => ("remove", None),
        TrimAction::TrimTo(text) => ("trim", Some(text.clone())),
    };
    ReportPair {
        keep: pair.keep,
        keep_speaker: utterances[pair.keep].speaker.clone(),
        drop: pair.drop,
        drop_speaker: utterances[pair.drop].speaker.clone(),
        kind: match pair.kind {
            DuplicateKind::Exact => "exact",
            DuplicateKind::Near => "near",
        },
        similarity: pair.similarity,
        action,
        trimmed_text,
    }
}
