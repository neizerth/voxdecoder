//! Plain text body (single span).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtBody {
    pub text: String,
}

impl TxtBody {
    pub fn parse(raw: &str) -> Self {
        Self {
            text: raw.to_string(),
        }
    }

    pub fn serialize(&self) -> String {
        self.text.clone()
    }
}
