//! Domain: media filter-chain executor.

mod chain;
mod executor;
mod filter;
mod provider;
mod result;

pub use chain::{expand_and_validate, load_chain_file, ChainFile};
pub use executor::{
    execute, execute_with_progress, plan, request_from_raw, ExecutionPlan, PlannedFilter,
    PreprocessRequest,
};
pub use filter::{catalog_lines, parse_filter_flag, FilterGroup, FilterSpec, RawFilter};
pub use provider::{ffmpeg_argv_for_plan, MediaProviderSpec};
pub use result::{PreparedMedia, PreprocessResult};

#[derive(Debug, thiserror::Error)]
pub enum PreprocessError {
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Other(String),
}

impl PreprocessError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::NotFound(_) => 3,
            Self::Unavailable(_) | Self::Other(_) => 1,
        }
    }
}
