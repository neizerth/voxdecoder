//! Structured glossary parsing (yaml / json / markdown arrows).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermEntry {
    pub canonical: String,
    #[serde(default)]
    pub variants: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawTerm {
    canonical: String,
    #[serde(default)]
    variants: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DictFile {
    #[serde(default)]
    entries: Vec<RawTerm>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum JsonRoot {
    List(Vec<RawTerm>),
    Wrapped { terms: Vec<RawTerm> },
    Dict(DictFile),
    Single(RawTerm),
}

pub fn parse_any(text: &str) -> Result<Vec<TermEntry>, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    if let Ok(entries) = parse_yaml(trimmed) {
        if !entries.is_empty() {
            return Ok(entries);
        }
    }
    if let Ok(entries) = parse_json(trimmed) {
        if !entries.is_empty() {
            return Ok(entries);
        }
    }
    let md = parse_markdown_arrows(trimmed);
    if md.is_empty() {
        // DictFile yaml with only forms
        if let Ok(file) = serde_yaml::from_str::<DictFile>(trimmed) {
            return Ok(file.entries.into_iter().map(into_entry).collect());
        }
        return Ok(Vec::new());
    }
    Ok(md)
}

fn parse_yaml(text: &str) -> Result<Vec<TermEntry>, String> {
    if let Ok(list) = serde_yaml::from_str::<Vec<RawTerm>>(text) {
        return Ok(list.into_iter().map(into_entry).collect());
    }
    if let Ok(file) = serde_yaml::from_str::<DictFile>(text) {
        if !file.entries.is_empty() {
            return Ok(file.entries.into_iter().map(into_entry).collect());
        }
    }
    let mut entries = Vec::new();
    for doc in split_docs(text) {
        if doc.is_empty() {
            continue;
        }
        if let Ok(raw) = serde_yaml::from_str::<RawTerm>(doc) {
            entries.push(into_entry(raw));
        } else if let Ok(list) = serde_yaml::from_str::<Vec<RawTerm>>(doc) {
            entries.extend(list.into_iter().map(into_entry));
        }
    }
    if entries.is_empty() {
        Err("no yaml glossary entries".into())
    } else {
        Ok(entries)
    }
}

fn parse_json(text: &str) -> Result<Vec<TermEntry>, String> {
    let root: JsonRoot = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(match root {
        JsonRoot::List(list) => list.into_iter().map(into_entry).collect(),
        JsonRoot::Wrapped { terms } => terms.into_iter().map(into_entry).collect(),
        JsonRoot::Dict(d) => d.entries.into_iter().map(into_entry).collect(),
        JsonRoot::Single(t) => vec![into_entry(t)],
    })
}

fn parse_markdown_arrows(text: &str) -> Vec<TermEntry> {
    let mut entries = Vec::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches(['-', '*', '+']).trim();
        if let Some((left, right)) = split_arrow(line) {
            let variant = strip_ticks(left);
            let canonical = strip_ticks(right);
            if !variant.is_empty() && !canonical.is_empty() {
                entries.push(TermEntry {
                    canonical,
                    variants: vec![variant],
                });
            }
        }
    }
    entries
}

fn split_docs(text: &str) -> Vec<&str> {
    let parts: Vec<&str> = text
        .split("\n---\n")
        .map(|p| p.trim().trim_start_matches("---").trim())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        vec![text.trim()]
    } else {
        parts
    }
}

fn split_arrow(line: &str) -> Option<(&str, &str)> {
    for sep in ["→", "->", "=>"] {
        if let Some((l, r)) = line.split_once(sep) {
            return Some((l.trim(), r.trim()));
        }
    }
    None
}

fn strip_ticks(s: &str) -> String {
    s.trim()
        .trim_matches('`')
        .trim_matches('*')
        .trim()
        .to_string()
}

fn into_entry(raw: RawTerm) -> TermEntry {
    TermEntry {
        canonical: raw.canonical,
        variants: raw.variants,
    }
}
