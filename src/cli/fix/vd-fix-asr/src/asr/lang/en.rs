//! Builtin en ASR-mistake dictionary, migrated from the legacy lexicon
//! backend (recognition phonetics, not project-canonical names).

pub const BUILTIN: &[(&str, &str)] = &[
    ("githap", "github"),
    ("githapp", "github"),
    ("gitub", "github"),
    ("actoins", "actions"),
    ("aciton", "action"),
    ("kubernetis", "kubernetes"),
    ("safetensores", "safetensors"),
];
