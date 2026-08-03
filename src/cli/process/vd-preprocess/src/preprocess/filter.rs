//! FilterSpec · groups · CLI sugar.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::PreprocessError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterGroup {
    Media,
    Audio,
    Timing,
    Channels,
}

/// One step in the filter chain.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterSpec {
    pub provider: String,
    pub operation: String,
    #[serde(default, flatten)]
    pub params: BTreeMap<String, serde_json::Value>,
}

/// Raw YAML item before sugar expand.
#[derive(Debug, Clone, Deserialize)]
pub struct RawFilter {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub operation: Option<String>,
    /// Sugar: `type: X` ≡ default provider + `operation: X`.
    #[serde(default, rename = "type")]
    pub r#type: Option<String>,
    #[serde(flatten)]
    pub params: BTreeMap<String, serde_json::Value>,
}

impl RawFilter {
    pub fn expand(self, default_provider: &str) -> Result<FilterSpec, PreprocessError> {
        let mut params = self.params;
        params.remove("provider");
        params.remove("operation");
        params.remove("type");

        let (provider, operation) = if let Some(op) = self.operation {
            let provider = self
                .provider
                .unwrap_or_else(|| default_provider.to_string());
            (provider, op)
        } else if let Some(t) = self.r#type {
            let provider = self
                .provider
                .unwrap_or_else(|| default_provider.to_string());
            (provider, t)
        } else {
            return Err(PreprocessError::Usage(
                "filter needs operation or type".into(),
            ));
        };

        if provider.is_empty() || operation.is_empty() {
            return Err(PreprocessError::Usage(
                "filter provider/operation must be non-empty".into(),
            ));
        }

        Ok(FilterSpec {
            provider,
            operation,
            params,
        })
    }
}

pub fn group_for(operation: &str) -> Option<FilterGroup> {
    match operation {
        "extract-audio" | "convert" | "resample" | "mono" | "stereo" => Some(FilterGroup::Media),
        "normalize" | "denoise" | "enhance" | "highpass" | "lowpass" | "compressor" => {
            Some(FilterGroup::Audio)
        }
        "speed" | "trim-silence" | "trim" | "chunk" | "pad-start" | "pad-end" => {
            Some(FilterGroup::Timing)
        }
        "split-channels" | "merge-channels" => Some(FilterGroup::Channels),
        _ => None,
    }
}

pub fn known_operation(operation: &str) -> bool {
    group_for(operation).is_some()
}

/// Parse `--filter name` or `name:key=val,key2=val2`.
pub fn parse_filter_flag(spec: &str, default_provider: &str) -> Result<FilterSpec, PreprocessError> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(PreprocessError::Usage("empty --filter".into()));
    }
    let (name, rest) = match spec.split_once(':') {
        Some((n, r)) => (n.trim(), Some(r.trim())),
        None => (spec, None),
    };
    if name.is_empty() {
        return Err(PreprocessError::Usage(format!("bad --filter: {spec}")));
    }
    let mut params = BTreeMap::new();
    if let Some(rest) = rest {
        for part in rest.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let (k, v) = part.split_once('=').ok_or_else(|| {
                PreprocessError::Usage(format!("bad --filter param (need key=val): {part}"))
            })?;
            let k = k.trim();
            let v = v.trim();
            if k.is_empty() {
                return Err(PreprocessError::Usage(format!("bad --filter: {spec}")));
            }
            let json_v = parse_param_value(v);
            params.insert(k.to_string(), json_v);
        }
    }
    Ok(FilterSpec {
        provider: default_provider.to_string(),
        operation: name.to_string(),
        params,
    })
}

fn parse_param_value(v: &str) -> serde_json::Value {
    if let Ok(n) = v.parse::<f64>() {
        return serde_json::json!(n);
    }
    if matches!(v, "true" | "false") {
        return serde_json::json!(v == "true");
    }
    serde_json::json!(v)
}

pub fn catalog_lines() -> Vec<String> {
    let ops = [
        ("Media", &["extract-audio", "convert", "resample", "mono", "stereo"][..]),
        (
            "Audio",
            &["normalize", "denoise", "enhance", "highpass", "lowpass", "compressor"][..],
        ),
        (
            "Timing",
            &["speed", "trim-silence", "trim", "chunk", "pad-start", "pad-end"][..],
        ),
        ("Channels", &["split-channels", "merge-channels"][..]),
    ];
    let mut lines = Vec::new();
    for (group, list) in ops {
        lines.push(format!("{group}:"));
        for op in list {
            lines.push(format!("  {op}"));
        }
    }
    lines
}
