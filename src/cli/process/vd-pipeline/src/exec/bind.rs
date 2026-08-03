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
}

#[derive(Debug, Clone)]
pub struct InvokeResult {
    pub primary_output: PathBuf,
    pub outputs: BTreeMap<String, PathBuf>,
}

pub trait Binder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError>;
}
