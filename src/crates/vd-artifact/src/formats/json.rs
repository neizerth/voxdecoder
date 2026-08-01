//! JSON / JSONL: mutate known transcript string keys only.

use serde_json::Value;

/// Case-insensitive transcript field names (structure / ids / timestamps stay untouched).
const TEXT_KEYS: &[&str] = &[
    "text",
    "content",
    "transcript",
    "utterance",
    "caption",
    "sentence",
    "line",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonBody {
    pub value: Value,
}

impl JsonBody {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        Ok(Self { value })
    }

    pub fn serialize(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.value).map_err(|e| e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonlBody {
    pub lines: Vec<Value>,
}

impl JsonlBody {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut lines = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let value: Value =
                serde_json::from_str(line).map_err(|e| format!("jsonl line {}: {e}", i + 1))?;
            lines.push(value);
        }
        Ok(Self { lines })
    }

    pub fn serialize(&self) -> Result<String, String> {
        let mut out = String::new();
        for v in &self.lines {
            out.push_str(&serde_json::to_string(v).map_err(|e| e.to_string())?);
            out.push('\n');
        }
        Ok(out)
    }
}

pub fn is_text_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    TEXT_KEYS.iter().any(|k| *k == lower)
}
