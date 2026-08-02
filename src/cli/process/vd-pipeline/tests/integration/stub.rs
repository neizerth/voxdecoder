//! Stub capability binder for integration tests.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use vd_pipeline::{ArgValue, Binder, Capability, ExecError, InvokeRequest, InvokeResult};

#[derive(Debug, Clone)]
pub struct RecordedCall {
    pub capability: Capability,
    pub input: PathBuf,
    pub options: BTreeMap<String, ArgValue>,
}

pub struct RecordingBinder {
    pub calls: Mutex<Vec<RecordedCall>>,
    pub fail_on: Option<Capability>,
    counter: AtomicUsize,
}

impl RecordingBinder {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fail_on: None,
            counter: AtomicUsize::new(0),
        }
    }

    pub fn failing(cap: Capability) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            fail_on: Some(cap),
            counter: AtomicUsize::new(0),
        }
    }

    fn invoke_inner(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        self.calls.lock().unwrap().push(RecordedCall {
            capability: req.capability,
            input: req.input.clone(),
            options: req.options.clone(),
        });
        if self.fail_on == Some(req.capability) {
            return Err(ExecError::Step(format!(
                "stub fail: {}",
                req.capability.as_str()
            )));
        }
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        let out = req
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("/tmp/stub-out-{n}.txt")));
        Ok(InvokeResult {
            primary_output: out,
            outputs: BTreeMap::new(),
        })
    }
}

impl Binder for RecordingBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        self.invoke_inner(req)
    }
}

impl Binder for &RecordingBinder {
    fn invoke(&self, req: &InvokeRequest) -> Result<InvokeResult, ExecError> {
        self.invoke_inner(req)
    }
}

pub fn nodes(steps: Vec<vd_pipeline::Step>) -> Vec<vd_pipeline::WorkflowNode> {
    steps.into_iter().map(Into::into).collect()
}
