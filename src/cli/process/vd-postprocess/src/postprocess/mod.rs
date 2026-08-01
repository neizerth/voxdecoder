//! Domain: recipe executor.

pub mod recipe;

mod executor;
mod provider;
mod result;

pub use executor::{
    execute, plan, ArtifactBinding, ExecutionPlan, PlannedRecipe, PostprocessRequest,
};
pub use provider::ExecutionProviderSpec;
pub use recipe::RecipeDoc;
pub use result::{DerivedArtifact, PostprocessResult, RecipeResult};

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
