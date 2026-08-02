//! Shared InputSource (XOR of path | uri | url | artifact | blob).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::InputError;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

impl InputSource {
    pub fn supplied_count(&self) -> usize {
        [
            self.path.is_some(),
            self.uri.is_some(),
            self.url.is_some(),
            self.artifact.is_some(),
            self.blob.is_some(),
        ]
        .into_iter()
        .filter(|p| *p)
        .count()
    }

    pub fn validate_xor(&self) -> Result<(), InputError> {
        if self.supplied_count() != 1 {
            return Err(InputError::Invalid(
                "input must specify exactly one of path, uri, url, artifact, or blob".into(),
            ));
        }
        Ok(())
    }

    pub fn as_url(&self) -> Option<&str> {
        self.url.as_deref().filter(|u| !u.is_empty())
    }
}
