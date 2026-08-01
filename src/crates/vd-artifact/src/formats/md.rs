//! Markdown body (single span — whole file).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MdBody {
    pub text: String,
}

impl MdBody {
    pub fn parse(raw: &str) -> Self {
        Self {
            text: raw.to_string(),
        }
    }

    pub fn serialize(&self) -> String {
        self.text.clone()
    }
}
