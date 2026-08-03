//! Runtime-owned domain request planning (resolve via `vd-input`, then plan).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vd_input::{resolve, ResolveContext, SubtitlePolicy};
pub use vd_input::InputSource;
use vd_meeting::{
    plan_job, BuildOptions, InputPurpose, InputRole, MeetingModel, MeetingOutput, MeetingRequest,
    TranscribeDefaults,
};
use vd_pipeline::{default_job, DefaultJobArgs, Job, TranscribeEngine};

use crate::store::JobStore;

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
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub subtitles: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub docs: Option<PathBuf>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
    #[serde(default)]
    pub continue_on_error: bool,
    #[serde(default = "default_true")]
    pub overwrite: bool,
}

fn default_true() -> bool {
    true
}

fn lookup<'a>(
    store: Option<&'a JobStore>,
) -> Option<Box<dyn Fn(&str) -> Result<PathBuf, String> + 'a>> {
    store.map(|s| {
        Box::new(move |id: &str| s.resolve_artifact(id).map_err(|e| e.to_string()))
            as Box<dyn Fn(&str) -> Result<PathBuf, String> + 'a>
    })
}

pub fn plan_audio(
    request: &AudioRequest,
    data_dir: &Path,
    store: Option<&JobStore>,
) -> Result<Job, PlanError> {
    if let Some(speed) = request.speed {
        if !(0.25..=4.0).contains(&speed) {
            return Err(PlanError::InvalidInput(format!(
                "speed must be between 0.25 and 4.0 (got {speed})"
            )));
        }
    }

    let mut ctx = ResolveContext::new(data_dir);
    ctx.overwrite = request.overwrite;
    ctx.provider_hint = request.provider.as_deref();
    if let Some(s) = &request.subtitles {
        ctx.subtitles = SubtitlePolicy::parse(s).map_err(PlanError::InvalidInput)?;
    }

    let boxed = lookup(store);
    let lookup_ref = boxed.as_ref().map(|b| b.as_ref() as &dyn Fn(&str) -> Result<PathBuf, String>);
    let resolved = resolve(&request.audio, &ctx, lookup_ref)
        .map_err(|e| PlanError::InvalidInput(e.to_string()))?;
    let audio = resolved
        .require_audio()
        .map_err(|e| PlanError::InvalidInput(e.to_string()))?
        .clone();

    let engine = match request.engine.as_deref() {
        Some(engine) => TranscribeEngine::parse(engine).ok_or_else(|| {
            PlanError::InvalidInput(format!("unknown transcription engine: {engine}"))
        })?,
        None => TranscribeEngine::Gigaam,
    };

    Ok(default_job(&DefaultJobArgs {
        audio,
        engine,
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
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub purposes: Vec<InputPurpose>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitles: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl MeetingInput {
    fn to_wire_source(&self) -> InputSource {
        InputSource {
            path: self.path.clone(),
            uri: self.uri.clone(),
            url: self.url.clone(),
            artifact: self.artifact.clone(),
            blob: self.blob.clone(),
        }
    }

    fn to_meeting_source(
        &self,
        data_dir: &Path,
        store: Option<&JobStore>,
    ) -> Result<vd_meeting::InputSource, PlanError> {
        let wire = self.to_wire_source();
        wire.validate_xor()
            .map_err(|e| PlanError::InvalidInput(e.to_string()))?;

        if self.role == InputRole::Context {
            if wire.as_url().is_some() {
                return Err(PlanError::InvalidInput(
                    "context inputs cannot use url (use path/uri for docs)".into(),
                ));
            }
            let boxed = lookup(store);
            let lookup_ref = boxed
                .as_ref()
                .map(|b| b.as_ref() as &dyn Fn(&str) -> Result<PathBuf, String>);
            let resolved = resolve(&wire, &ResolveContext::new(data_dir), lookup_ref)
                .map_err(|e| PlanError::InvalidInput(e.to_string()))?;
            let path = resolved
                .audio
                .or(resolved.metadata)
                .ok_or_else(|| PlanError::InvalidInput("context input unresolved".into()))?;
            return Ok(vd_meeting::InputSource {
                role: self.role,
                path,
                url: None,
                participant: self.participant.clone(),
                purposes: self.purposes.clone(),
                subtitles: None,
                provider: None,
            });
        }

        if wire.as_url().is_some() {
            return Ok(vd_meeting::InputSource {
                role: self.role,
                path: PathBuf::new(),
                url: wire.url.clone(),
                participant: self.participant.clone(),
                purposes: self.purposes.clone(),
                subtitles: self.subtitles.clone(),
                provider: self.provider.clone(),
            });
        }

        let boxed = lookup(store);
        let lookup_ref = boxed
            .as_ref()
            .map(|b| b.as_ref() as &dyn Fn(&str) -> Result<PathBuf, String>);
        let resolved = resolve(&wire, &ResolveContext::new(data_dir), lookup_ref)
            .map_err(|e| PlanError::InvalidInput(e.to_string()))?;
        Ok(vd_meeting::InputSource {
            role: self.role,
            path: resolved
                .require_audio()
                .map_err(|e| PlanError::InvalidInput(e.to_string()))?
                .clone(),
            url: None,
            participant: self.participant.clone(),
            purposes: self.purposes.clone(),
            subtitles: None,
            provider: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<InputSource>,
    #[serde(default)]
    pub options: BuildOptions,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Preprocess playback speed (e.g. 1.5 / 2.0 / 2.2). Remapped via TimeMap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
    /// Accompanying documents (folder or file) → `role: context` for prepare-context / fix-*.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<PathBuf>,
    /// Overwrite existing prepared / transcript / meeting artifacts (default false = reuse).
    #[serde(default)]
    pub overwrite: bool,
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
    if request.overwrite {
        options.transcribe.overwrite = true;
    }
    if request.engine.is_some()
        || request.model.is_some()
        || request.device.is_some()
        || request.speed.is_some()
    {
        options.transcribe = TranscribeDefaults {
            engine: request.engine.clone().or(options.transcribe.engine),
            model: request.model.clone().or(options.transcribe.model),
            device: request.device.clone().or(options.transcribe.device),
            speed: request.speed.or(options.transcribe.speed),
            overwrite: options.transcribe.overwrite,
        };
    }
    if let Some(factor) = options.transcribe.speed {
        if !(0.25..=4.0).contains(&factor) {
            return Err(PlanError::InvalidInput(format!(
                "speed must be between 0.25 and 4.0 (got {factor})"
            )));
        }
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
        inputs.push(input.to_meeting_source(data_dir, store)?);
    }
    if inputs.is_empty() {
        let audio = request.audio.as_ref().ok_or_else(|| {
            PlanError::InvalidInput(
                "meeting requires inputs, audio, document, or meeting_yaml".into(),
            )
        })?;
        audio
            .validate_xor()
            .map_err(|e| PlanError::InvalidInput(e.to_string()))?;
        if let Some(url) = audio.as_url() {
            inputs.push(vd_meeting::InputSource {
                role: InputRole::Room,
                path: PathBuf::new(),
                url: Some(url.to_string()),
                participant: None,
                purposes: Vec::new(),
                subtitles: None,
                provider: None,
            });
        } else {
            let boxed = lookup(store);
            let lookup_ref = boxed
                .as_ref()
                .map(|b| b.as_ref() as &dyn Fn(&str) -> Result<PathBuf, String>);
            let resolved = resolve(audio, &ResolveContext::new(data_dir), lookup_ref)
                .map_err(|e| PlanError::InvalidInput(e.to_string()))?;
            inputs.push(vd_meeting::InputSource {
                role: InputRole::Room,
                path: resolved
                    .require_audio()
                    .map_err(|e| PlanError::InvalidInput(e.to_string()))?
                    .clone(),
                url: None,
                participant: None,
                purposes: Vec::new(),
                subtitles: None,
                provider: None,
            });
        }
    }

    if let Some(docs) = &request.docs {
        let has_context = inputs.iter().any(|i| i.role == InputRole::Context);
        if !has_context {
            inputs.push(vd_meeting::InputSource {
                role: InputRole::Context,
                path: docs.clone(),
                url: None,
                participant: None,
                purposes: Vec::new(),
                subtitles: None,
                provider: None,
            });
        }
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
