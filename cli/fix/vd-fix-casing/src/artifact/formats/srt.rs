//! SRT: cue text only; timestamps / indices untouched.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrtCue {
    pub index: String,
    pub timing: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrtBody {
    pub cues: Vec<SrtCue>,
}

impl SrtBody {
    pub fn parse(raw: &str) -> Self {
        let normalized = raw.replace("\r\n", "\n");
        let mut cues = Vec::new();
        for block in normalized.split("\n\n") {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }
            let mut lines = block.lines();
            let Some(index) = lines.next() else {
                continue;
            };
            let Some(timing) = lines.next() else {
                continue;
            };
            let text: String = lines.collect::<Vec<_>>().join("\n");
            cues.push(SrtCue {
                index: index.to_string(),
                timing: timing.to_string(),
                text,
            });
        }
        Self { cues }
    }

    pub fn serialize(&self) -> String {
        let mut out = String::new();
        for (i, cue) in self.cues.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&cue.index);
            out.push('\n');
            out.push_str(&cue.timing);
            out.push('\n');
            out.push_str(&cue.text);
            out.push('\n');
        }
        out
    }
}
