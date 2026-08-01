//! Stderr progress for long-running CLI work (`--progress=text|json`).

mod progress;

pub use progress::{Progress, ProgressEvent, ProgressMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressFormat {
    Text,
    Json,
}

impl ProgressFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
        }
    }
}
