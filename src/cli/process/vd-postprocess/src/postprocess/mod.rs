//! Domain: recipe-graph executor.

pub mod recipe;

mod executor;
mod result;
mod runner;

pub use executor::{
    execute, execute_with_progress, plan, ArtifactBinding, ArtifactOutput, ExecutionNode,
    ExecutionPlan, InputBinding, PlannedOutput, PlannedRecipe, PostprocessRequest,
};
pub use recipe::RecipeDoc;
pub use result::{DerivedArtifact, PostprocessResult, RecipeResult};
pub use runner::{
    resolve_provider, resolve_runner, validate_provider_type, validate_runner_type,
    ExecutionProvider, ExecutionProviderSpec, ExecutionRunner, ProviderInvoke, RunnerInvoke,
    RunnerSpec,
};

#[derive(Debug, thiserror::Error)]
pub enum PostprocessError {
    #[error("{0}")]
    Usage(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unavailable(String),
    #[error("{0}")]
    Other(String),
}

impl PostprocessError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::NotFound(_) => 3,
            Self::Unavailable(_) | Self::Other(_) => 1,
        }
    }
}
