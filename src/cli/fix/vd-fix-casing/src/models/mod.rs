//! Language packs (installable models).

mod catalog;
mod pack;

pub use catalog::{
    catalog_help_lines, is_catalog_name, resolve_model_name, shipping_names, CatalogEntry, CATALOG,
};
pub use pack::{
    info, install, is_installed, list_status, pack_dir, remove, resolve_lexicon, InstallOutcome,
    Lexicon, ModelInfo, ModelStatus, PackError,
};
