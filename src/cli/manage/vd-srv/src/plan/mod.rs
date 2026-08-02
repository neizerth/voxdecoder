//! Runtime-owned domain request planning.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use vd_meeting::{
    plan_job, BuildOptions, InputPurpose, InputRole, MeetingModel, MeetingOutput, MeetingRequest,
    TranscribeDefaults,
};
use vd_pipeline::{default_job, DefaultJobArgs, Job, TranscribeEngine};

use crate::store::JobStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
}

impl InputSource {
    pub fn resolve(&self, data_dir: &Path, store: Option<&JobStore>) -> Result<PathBuf, PlanError> {
        let supplied = [
            self.path.is_some(),
            self.uri.is_some(),
            self.artifact.is_some(),
            self.blob.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        if supplied != 1 {
            return Err(PlanError::InvalidInput(
                "input must specify exactly one of path, uri, artifact, or blob".into(),
            ));
        }
        if let Some(path) = &self.path {
            return Ok(path.clone());
        }
        if let Some(uri) = &self.uri {
            return uri
                .strip_prefix("file://")
                .map(PathBuf::from)
                .ok_or_else(|| {
                    PlanError::InvalidInput(format!("unsupported input URI scheme: {uri}"))
                });
        }
        if let Some(artifact) = &self.artifact {
            let store = store.ok_or_else(|| {
                PlanError::InvalidInput("artifact inputs require a Runtime Job Store".into())
            })?;
            return store
                .resolve_artifact(artifact)
                .map_err(|e| PlanError::InvalidInput(e.to_string()));
        }
        let blob = self.blob.as_deref().unwrap_or_default();
        let dir = data_dir.join("inputs");
        fs::create_dir_all(&dir).map_err(|e| PlanError::Io(e.to_string()))?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| PlanError::Io(e.to_string()))?
            .as_nanos();
        let path = dir.join(format!("blob-{nonce}.bin"));
        fs::write(&path, blob.as_bytes()).map_err(|e| PlanError::Io(e.to_string()))?;
        Ok(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRequest {
    pub audio: InputSource,
    #[serde(default)]
    pub engine: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub flash: bool,
    /// Preprocess playback speed (e.g. 2.0–2.2). Timestamps remapped via TimeMap.
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub docs: Option<PathBuf>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub continue_on_error: bool,
    /// When unset, Runtime defaults to overwriting outputs next to the source.
    #[serde(default = "default_true")]
    pub overwrite: bool,
}

fn default_true() -> bool {
    true
}

pub fn plan_audio(
    request: &AudioRequest,
    data_dir: &Path,
    store: Option<&JobStore>,
) -> Result<Job, PlanError> {
    let audio = request.audio.resolve(data_dir, store)?;
    if let Some(speed) = request.speed {
        if !(0.25..=4.0).contains(&speed) {
            return Err(PlanError::InvalidInput(format!(
                "speed must be between 0.25 and 4.0 (got {speed})"
            )));
        }
    }
    Ok(default_job(&DefaultJobArgs {
        audio,
        engine: match request.engine.as_deref() {
            Some(engine) => TranscribeEngine::parse(engine).ok_or_else(|| {
                PlanError::InvalidInput(format!("unknown transcription engine: {engine}"))
            })?,
            None => TranscribeEngine::Gigaam,
        },
        model: request.model.clone(),
        device: request.device.clone(),
        flash: request.flash,
        speed: request.speed,
        docs: request.docs.clone(),
        output_dir: request.output_dir.clone(),
        working_dir: request.working_dir.clone(),
        continue_on_error: request.continue_on_error,
        overwrite: request.overwrite,
    }))
}

/// Runtime-facing meeting input: role + shared InputSource fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingInput {
    pub role: InputRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub purposes: Vec<InputPurpose>,
}

impl MeetingInput {
    fn to_source(&self) -> InputSource {
        InputSource {
            path: self.path.clone(),
            uri: self.uri.clone(),
            artifact: self.artifact.clone(),
            blob: self.blob.clone(),
        }
    }

    fn resolve(
        &self,
        data_dir: &Path,
        store: Option<&JobStore>,
    ) -> Result<vd_meeting::InputSource, PlanError> {
        Ok(vd_meeting::InputSource {
            role: self.role,
            path: self.to_source().resolve(data_dir, store)?,
            participant: self.participant.clone(),
            purposes: self.purposes.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingPlanRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub inputs: Vec<MeetingInput>,
    #[serde(default)]
    pub meeting: MeetingModel,
    #[serde(default)]
    pub output: MeetingOutput,
    /// Convenience: single room input when `inputs` is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<InputSource>,
    #[serde(default)]
    pub options: BuildOptions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meeting_yaml: Option<String>,
}

pub fn plan_meeting(
    request: &MeetingPlanRequest,
    data_dir: &Path,
    store: Option<&JobStore>,
) -> Result<Job, PlanError> {
    let (meeting_req, mut options) = materialize_meeting(request, data_dir, store)?;
    if request.engine.is_some() || request.model.is_some() {
        options.transcribe = TranscribeDefaults {
            engine: request.engine.clone().or(options.transcribe.engine),
            model: request.model.clone().or(options.transcribe.model),
            overwrite: options.transcribe.overwrite,
        };
    }
    plan_job(&meeting_req, &options).map_err(|e| PlanError::InvalidInput(e.to_string()))
}

fn materialize_meeting(
    request: &MeetingPlanRequest,
    data_dir: &Path,
    store: Option<&JobStore>,
) -> Result<(MeetingRequest, BuildOptions), PlanError> {
    if let Some(raw) = request
        .document
        .as_deref()
        .or(request.meeting_yaml.as_deref())
    {
        let (meeting, options) = parse_meeting_document(raw)?;
        return Ok((meeting, options.unwrap_or_else(|| request.options.clone())));
    }

    let mut inputs = Vec::with_capacity(request.inputs.len().max(1));
    for input in &request.inputs {
        inputs.push(input.resolve(data_dir, store)?);
    }
    if inputs.is_empty() {
        let audio = request.audio.as_ref().ok_or_else(|| {
            PlanError::InvalidInput(
                "meeting requires inputs, audio, document, or meeting_yaml".into(),
            )
        })?;
        inputs.push(vd_meeting::InputSource {
            role: InputRole::Room,
            path: audio.resolve(data_dir, store)?,
            participant: None,
            purposes: Vec::new(),
        });
    }

    Ok((
        MeetingRequest {
            working_dir: request.working_dir.clone(),
            inputs,
            meeting: request.meeting.clone(),
            output: request.output.clone(),
        },
        request.options.clone(),
    ))
}

fn parse_meeting_document(raw: &str) -> Result<(MeetingRequest, Option<BuildOptions>), PlanError> {
    use vd_meeting::model::MeetingDocument;

    let trimmed = raw.trim_start();
    let doc: MeetingDocument = if trimmed.starts_with('{') {
        serde_json::from_str(trimmed).map_err(|e| PlanError::InvalidInput(e.to_string()))?
    } else {
        serde_yaml::from_str(raw).map_err(|e| PlanError::InvalidInput(e.to_string()))?
    };
    if doc.version != 1 {
        return Err(PlanError::InvalidInput(format!(
            "unsupported meeting document version: {}",
            doc.version
        )));
    }
    if doc.inputs.is_empty() {
        return Err(PlanError::InvalidInput(
            "meeting document has no inputs".into(),
        ));
    }
    let build = doc.build.clone();
    Ok((doc.into_request(), build))
}

/// `execute` (default true) with `run` as alias.
pub fn wants_execute(params: &serde_json::Value) -> bool {
    params
        .get("execute")
        .or_else(|| params.get("run"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true)
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("{0}")]
    Io(String),
}
