//! Output path resolution for transcript CLIs (filesystem only).

mod path;

pub use path::{resolve_output_path, OutputPathError, OutputPathRequest, OutputPaths};
