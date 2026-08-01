//! Lexicon authority — shipping + `--terms` via `vd-assets` project assets.

mod merge;
mod shipping;

use std::collections::HashMap;

use crate::terms::TermsLoadOptions;

pub use merge::TermEntry;

/// Merged variant → canonical map. Read-only after load.
#[derive(Debug, Clone)]
pub struct Lexicon {
    map: HashMap<String, String>,
    matchers: Vec<(String, String)>,
}

#[derive(Debug, thiserror::Error)]
pub enum LexiconError {
    #[error("--terms path missing / unreadable: {0}")]
    MissingPath(String),
    #[error("failed to read terms source: {0}")]
    Io(String),
    #[error("lexicon failed to initialize: {0}")]
    Init(String),
}

impl LexiconError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::MissingPath(_) | Self::Io(_) => 3,
            Self::Init(_) => 4,
        }
    }
}

impl Lexicon {
    /// Load shipping lexicon (optional) then `--terms` paths (**last wins**).
    ///
    /// Prefer `--terms ./assets` from `vd-assets` (`terms.yml` + `md/`).
    pub fn load(opts: &TermsLoadOptions) -> Result<Self, LexiconError> {
        let mut map = HashMap::new();

        if opts.shipping {
            merge::apply_entries(&mut map, shipping::entries(opts.language));
        }

        if !opts.terms_paths.is_empty() {
            let dict = vd_assets::load_dictionary(
                &opts.terms_paths,
                &vd_assets::DictionaryOptions::default(),
            )
            .map_err(|e| match e.exit_code() {
                3 => LexiconError::MissingPath(e.to_string()),
                _ => LexiconError::Init(e.to_string()),
            })?;
            let entries: Vec<TermEntry> = dict
                .term_entries_for_fixer()
                .into_iter()
                .map(|e| TermEntry {
                    canonical: e.canonical,
                    variants: e.variants,
                })
                .collect();
            merge::apply_entries(&mut map, entries);
        }

        Ok(Self::from_map(map))
    }

    pub fn from_entries(entries: Vec<TermEntry>) -> Self {
        let mut map = HashMap::new();
        merge::apply_entries(&mut map, entries);
        Self::from_map(map)
    }

    fn from_map(map: HashMap<String, String>) -> Self {
        let mut matchers: Vec<(String, String)> =
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        matchers.sort_by_key(|b| std::cmp::Reverse(b.0.chars().count()));
        Self { map, matchers }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn canonical_for(&self, variant: &str) -> Option<&str> {
        self.map.get(&normalize_key(variant)).map(String::as_str)
    }

    pub(crate) fn matchers(&self) -> &[(String, String)] {
        &self.matchers
    }
}

pub(crate) fn normalize_key(s: &str) -> String {
    s.trim().to_lowercase()
}
