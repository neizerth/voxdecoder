#![allow(clippy::default_trait_access)]
//! Integration tests: Executor with stub binders.

mod exec_artifacts;
mod exec_chain;
mod exec_continue;
mod exec_options;
mod exec_order;
mod exec_prepare_context;
mod stub;

use stub::RecordingBinder;
