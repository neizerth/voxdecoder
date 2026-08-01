//! Output path resolution for VoxDecoder CLIs (filesystem only).

mod path;

pub use path::{
    ensure_writable, file_stem, fixed_file_name, resolve_output_path, segments_sidecar,
    stem_ext_file_name, OutputPathError, OutputPathRequest, OutputPaths,
};
