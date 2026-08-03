//! Load meeting YAML / JSON documents.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    BuildOptions, InputSource, MeetingModel, MeetingOutput, MeetingRequest,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeetingDocument {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<std::path::PathBuf>,
    pub inputs: Vec<InputSource>,
    #[serde(default)]
    pub meeting: MeetingModel,
    #[serde(default)]
    pub output: MeetingOutput,
    /// Optional BuildOptions embedded for MCP convenience (not Meeting Model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildOptions>,
}

impl MeetingDocument {
    pub fn into_request(self) -> MeetingRequest {
        MeetingRequest {
            working_dir: self.working_dir,
            inputs: self.inputs,
            meeting: self.meeting,
            output: self.output,
        }
    }
}

pub fn load_meeting_file(path: &Path) -> Result<(MeetingRequest, Option<BuildOptions>), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc = if path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    {
        serde_json::from_str::<MeetingDocument>(&text).map_err(|e| e.to_string())?
    } else {
        serde_yaml::from_str::<MeetingDocument>(&text).map_err(|e| e.to_string())?
    };
    if doc.version != 1 {
        return Err(format!("unsupported meeting document version: {}", doc.version));
    }
    if doc.inputs.is_empty() {
        return Err("meeting document has no inputs".into());
    }
    let build = doc.build.clone();
    Ok((doc.into_request(), build))
}
