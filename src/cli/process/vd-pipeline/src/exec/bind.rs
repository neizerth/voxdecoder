//! Capability binder trait.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::job::{ArgValue, Capability};

use super::ExecError;

#[derive(Debug, Clone)]
pub struct InvokeRequest {
    pub capability: Capability,
    /// Step / artifact id when set (e.g. `igor.cased`) — used by Runtime observe binding.
    pub step_id: Option<String>,
    pub working_dir: PathBuf,
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub context_assets: Option<PathBuf>,
    pub options: BTreeMap<String, ArgValue>,
    /// When set, child CLIs write live progress into this snapshot (Runtime `get_job`).
    pub progress_snapshot: Option<PathBuf>,
    /// Job-level percent at step start (for remapping child 0–100 into the step window).
    pub progress_step_base: Option<u8>,
    /// Percent span allocated to this step (typically `100 / n_steps`).
    pub progress_step_span: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct InvokeResult {
    pub primary_output: PathBuf,
    pub outputs: BTreeMap<String, PathBuf>,
}

pub trait Binder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError>;
}
