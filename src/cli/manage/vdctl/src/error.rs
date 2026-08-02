//! Platform CLI errors.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    NotReachable(String),
    #[error("{0}")]
    Workspace(String),
    #[error("{0}")]
    NotImplemented(String),
}

impl Error {
    pub fn message(&self) -> &str {
        match self {
            Self::Message(s)
            | Self::Usage(s)
            | Self::NotReachable(s)
            | Self::Workspace(s)
            | Self::NotImplemented(s) => s,
        }
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Message(_) | Self::NotImplemented(_) => 1,
            Self::Usage(_) => 2,
            Self::NotReachable(_) => 3,
            Self::Workspace(_) => 7,
        }
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::Message(value.to_string())
    }
}
