//! `vd-meeting run` / `plan`.

use std::path::PathBuf;

use super::{CliError, ProgressMode};
use crate::config;
use crate::model::{
    BuildOptions, DiarizationEnabled, InputRole, InputSource, MeetingModel, MeetingOutput,
    MeetingRequest,
};
use crate::paths;
use crate::planner::{require_paths, MeetingPlanner};
use crate::status;

#[derive(Debug, Clone)]
pub struct RunArgs {
    pub meeting_file: Option<PathBuf>,
    pub inputs: Vec<String>,
    pub context: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub asr: Option<String>,
    pub model: Option<String>,
    pub overwrite: bool,
    pub max_parallel: Option<u32>,
    pub continue_on_error: bool,
    pub dry_run: bool,
    pub json: bool,
    pub progress: Option<ProgressMode>,
    pub quiet: bool,
}

impl RunArgs {
    fn effective_progress(&self, file: Option<&str>) -> ProgressMode {
        if self.quiet {
            return ProgressMode::None;
        }
        if let Some(p) = self.progress {
            return p;
        }
        file.and_then(ProgressMode::parse)
            .unwrap_or(ProgressMode::Text)
    }
}

pub fn execute(args: RunArgs) -> Result<(), CliError> {
    let file_cfg = config::load(&paths::config_path()).map_err(CliError::usage)?;
    let (mut request, file_build) = assemble_request(&args)?;
    apply_config_defaults(&mut request, &file_cfg);

    let mut options = file_build.unwrap_or_default();
    merge_build_options(&mut options, &args, &file_cfg);

    let progress = status::start(args.effective_progress(file_cfg.progress.as_deref()));
    status::emit_phase(&progress, "collecting", 10);
    status::emit_phase(&progress, "normalizing", 30);

    let planned = MeetingPlanner::plan(&request, &options)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;

    if !args.dry_run {
        require_paths(&planned.resolved)
            .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
    }

    status::emit_phase(&progress, "planning", 60);
    let job = planned.job;

    if args.dry_run {
        if args.json {
            println!("{}", serde_json::to_string_pretty(&job).unwrap());
        } else {
            println!("{}", serde_yaml::to_string(&job).unwrap());
        }
        return Ok(());
    }

    status::emit_phase(&progress, "submitting", 80);
    let out = crate::planner::submit_job(job, ProgressMode::None)
        .map_err(|e| CliError::with_code(e.exit_code(), e.to_string()))?;
    println!("output: {}", out.display());
    Ok(())
}

fn assemble_request(args: &RunArgs) -> Result<(MeetingRequest, Option<BuildOptions>), CliError> {
    let mut request = MeetingRequest {
        working_dir: args.working_dir.clone(),
        inputs: Vec::new(),
        meeting: MeetingModel::default(),
        output: MeetingOutput::default(),
    };
    let mut build = None;

    if let Some(path) = &args.meeting_file {
        let (doc_req, doc_build) =
            crate::model::load_meeting_file(path).map_err(CliError::usage)?;
        request = doc_req;
        build = doc_build;
        if request.working_dir.is_none() {
            request.working_dir.clone_from(&args.working_dir);
        }
    }

    for spec in &args.inputs {
        request.inputs.push(parse_input_spec(spec)?);
    }
    if let Some(ctx) = &args.context {
        request.inputs.push(InputSource {
            role: InputRole::Context,
            path: ctx.clone(),
            participant: None,
        });
    }
    if let Some(dir) = &args.output_dir {
        request.output.dir = Some(dir.clone());
    }
    if request.inputs.is_empty() {
        return Err(CliError::usage("no inputs after assembling request"));
    }
    Ok((request, build))
}

fn parse_input_spec(spec: &str) -> Result<InputSource, CliError> {
    let mut role = None;
    let mut path = None;
    let mut participant = None;
    for part in spec.split(',') {
        let (k, v) = part
            .split_once('=')
            .ok_or_else(|| CliError::usage(format!("bad --input fragment: {part}")))?;
        match k.trim() {
            "role" => {
                role = Some(
                    InputRole::parse(v.trim())
                        .ok_or_else(|| CliError::usage(format!("unknown role: {v}")))?,
                );
            }
            "path" => path = Some(PathBuf::from(v.trim())),
            "participant" => participant = Some(v.trim().to_string()),
            other => {
                return Err(CliError::usage(format!("unknown --input key: {other}")));
            }
        }
    }
    Ok(InputSource {
        role: role.ok_or_else(|| CliError::usage("--input needs role=…"))?,
        path: path.ok_or_else(|| CliError::usage("--input needs path=…"))?,
        participant,
    })
}

fn apply_config_defaults(request: &mut MeetingRequest, cfg: &config::FileConfig) {
    if request.meeting.diarization.enabled == DiarizationEnabled::Auto {
        if let Some(s) = &cfg.diarization_enabled {
            if let Some(v) = DiarizationEnabled::parse(s) {
                request.meeting.diarization.enabled = v;
            }
        }
    }
    if let Some(s) = &cfg.alignment_mode {
        if let Some(m) = crate::model::AlignmentMode::parse(s) {
            if request.meeting.alignment.mode == crate::model::AlignmentMode::Longest {
                request.meeting.alignment.mode = m;
            }
        }
    }
}

fn merge_build_options(
    options: &mut BuildOptions,
    args: &RunArgs,
    cfg: &config::FileConfig,
) {
    if let Some(asr) = args.asr.clone().or_else(|| cfg.asr.clone()) {
        options.transcribe.engine = Some(asr);
    }
    if let Some(m) = &args.model {
        options.transcribe.model = Some(m.clone());
    }
    if args.overwrite {
        options.transcribe.overwrite = true;
    }
    if let Some(n) = args.max_parallel.or(cfg.max_parallel) {
        options.executor.max_parallel = Some(n);
    }
    if args.continue_on_error {
        options.executor.continue_on_error = true;
    }
}
