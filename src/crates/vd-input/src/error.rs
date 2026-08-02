//! Input resolution errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InputError {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Provider(String),
}
