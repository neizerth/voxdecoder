//! WebVTT: cue text only; timings / cue ids / header untouched.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VttBlock {
    Header(String),
    Cue {
        id: Option<String>,
        timing: String,
        text: String,
    },
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VttBody {
    pub blocks: Vec<VttBlock>,
}

impl VttBody {
    pub fn parse(raw: &str) -> Self {
        let normalized = raw.replace("\r\n", "\n");
        let mut blocks = Vec::new();
        let mut parts = normalized.split("\n\n");
        if let Some(first) = parts.next() {
            let first = first.trim_start_matches('\u{feff}');
            if first.starts_with("WEBVTT") {
                blocks.push(VttBlock::Header(first.to_string()));
            } else if !first.trim().is_empty() {
                blocks.push(parse_block(first));
            }
        }
        for part in parts {
            if part.trim().is_empty() {
                continue;
            }
            blocks.push(parse_block(part));
        }
        Self { blocks }
    }

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for (i, block) in self.blocks.iter().enumerate() {
            if i > 0 {
                out.push_str("\n\n");
            }
            match block {
                VttBlock::Header(h) | VttBlock::Other(h) => out.push_str(h),
                VttBlock::Cue { id, timing, text } => {
                    if let Some(id) = id {
                        out.push_str(id);
                        out.push('\n');
                    }
                    out.push_str(timing);
                    out.push('\n');
                    out.push_str(text);
                }
            }
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out
    }
}

fn parse_block(part: &str) -> VttBlock {
    let part = part.trim();
    let mut lines = part.lines();
    let Some(first) = lines.next() else {
        return VttBlock::Other(part.to_string());
    };
    if first.contains("-->") {
        let text: String = lines.collect::<Vec<_>>().join("\n");
        return VttBlock::Cue {
            id: None,
            timing: first.to_string(),
            text,
        };
    }
    if let Some(second) = lines.next() {
        if second.contains("-->") {
            let text: String = lines.collect::<Vec<_>>().join("\n");
            return VttBlock::Cue {
                id: Some(first.to_string()),
                timing: second.to_string(),
                text,
            };
        }
    }
    VttBlock::Other(part.to_string())
}
