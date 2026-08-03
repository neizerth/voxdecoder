//! `meeting-merge` capability step.

use std::collections::BTreeMap;

use vd_pipeline::{ArgValue, Capability, Step};

use super::overwrite_opt;
use crate::model::{BuildOptions, KnownParticipant, MeetingModel};
use crate::planner::normalize::ResolvedMeeting;
use crate::planner::PlanError;

pub fn append_merge(
    steps: &mut Vec<Step>,
    resolved: &ResolvedMeeting,
    text_ids: &[String],
    timeline_id: Option<&str>,
    mix_ref: Option<&str>,
    options: &BuildOptions,
) -> Result<(), PlanError> {
    let mut inputs: Vec<String> = text_ids.to_vec();
    if let Some(t) = timeline_id {
        inputs.push(t.to_string());
    }
    if let Some(m) = mix_ref {
        if !inputs.iter().any(|i| i == m) {
            inputs.push(m.to_string());
        }
    }

    let mut step = Step::new(Capability::MeetingMerge);
    step.id = Some("meeting".into());
    step.inputs = inputs;
    if let Some(dir) = &resolved.output.dir {
        step.output = Some(dir.join("meeting.json"));
    }

    let mut opts = overwrite_opt(options);
    opts.insert("alignment".into(), alignment_to_arg(&resolved.meeting));
    opts.insert("participants".into(), participants_to_arg(&resolved.meeting));
    opts.insert(
        "texts".into(),
        ArgValue::Strings(text_ids.iter().cloned().collect()),
    );
    if let Some(t) = timeline_id {
        opts.insert("timeline".into(), ArgValue::String(t.to_string()));
    }
    if let Some(m) = mix_ref {
        opts.insert("mix".into(), ArgValue::String(m.to_string()));
    }
    step.options = opts;
    steps.push(step);
    Ok(())
}

fn alignment_to_arg(m: &MeetingModel) -> ArgValue {
    let mut map = BTreeMap::new();
    map.insert(
        "mode".into(),
        ArgValue::String(m.alignment.mode.as_str().into()),
    );
    map.insert(
        "reference".into(),
        ArgValue::String(m.alignment.reference.as_str().into()),
    );
    if let Some(ms) = m.alignment.tolerance_ms {
        map.insert("tolerance_ms".into(), ArgValue::Number(f64::from(ms)));
    }
    if let Some(d) = m.alignment.allow_clock_drift {
        map.insert("allow_clock_drift".into(), ArgValue::Bool(d));
    }
    ArgValue::Map(map)
}

fn participants_to_arg(m: &MeetingModel) -> ArgValue {
    let mut map = BTreeMap::new();
    if !m.participants.known.is_empty() {
        let known: Vec<ArgValue> = m
            .participants
            .known
            .iter()
            .map(known_to_arg)
            .collect();
        let mut known_map = BTreeMap::new();
        for k in &m.participants.known {
            let id = k.id.clone().unwrap_or_else(|| "unknown".into());
            known_map.insert(id, known_to_arg(k));
        }
        map.insert("known".into(), ArgValue::Map(known_map));
        let _ = known;
    }
    if let Some(e) = &m.participants.expected {
        let mut b = BTreeMap::new();
        if let Some(v) = e.min {
            b.insert("min".into(), ArgValue::Number(f64::from(v)));
        }
        if let Some(v) = e.max {
            b.insert("max".into(), ArgValue::Number(f64::from(v)));
        }
        map.insert("expected".into(), ArgValue::Map(b));
    }
    if let Some(c) = &m.participants.constraints {
        let mut b = BTreeMap::new();
        if let Some(v) = c.min {
            b.insert("min".into(), ArgValue::Number(f64::from(v)));
        }
        if let Some(v) = c.max {
            b.insert("max".into(), ArgValue::Number(f64::from(v)));
        }
        if !c.genders.is_empty() {
            let mut gmap = BTreeMap::new();
            for (g, bounds) in &c.genders {
                let mut gb = BTreeMap::new();
                if let Some(v) = bounds.min {
                    gb.insert("min".into(), ArgValue::Number(f64::from(v)));
                }
                if let Some(v) = bounds.max {
                    gb.insert("max".into(), ArgValue::Number(f64::from(v)));
                }
                gmap.insert(g.as_str().into(), ArgValue::Map(gb));
            }
            b.insert("genders".into(), ArgValue::Map(gmap));
        }
        map.insert("constraints".into(), ArgValue::Map(b));
    }
    ArgValue::Map(map)
}

fn known_to_arg(k: &KnownParticipant) -> ArgValue {
    let mut m = BTreeMap::new();
    if let Some(id) = &k.id {
        m.insert("id".into(), ArgValue::String(id.clone()));
    }
    if let Some(name) = &k.name {
        m.insert("name".into(), ArgValue::String(name.clone()));
    }
    if k.optional {
        m.insert("optional".into(), ArgValue::Bool(true));
    }
    if let Some(g) = k.constraints.gender {
        let mut c = BTreeMap::new();
        c.insert("gender".into(), ArgValue::String(g.as_str().into()));
        m.insert("constraints".into(), ArgValue::Map(c));
    }
    ArgValue::Map(m)
}
