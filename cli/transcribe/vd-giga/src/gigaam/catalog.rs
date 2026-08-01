//! ASR catalog names, aliases, CDN URLs, checksums.

pub const CDN_BASE: &str = "https://cdn.chatwm.opensmodel.sberdevices.ru/GigaAM";

pub const CATALOG: &[&str] = &[
    "v3_e2e_rnnt",
    "v3_e2e_ctc",
    "v3_rnnt",
    "v3_ctc",
    "v2_rnnt",
    "v2_ctc",
    "v1_rnnt",
    "v1_ctc",
];

/// MD5 of the catalog `.ckpt` (from official GigaAM `_MODEL_HASHES`).
pub fn ckpt_md5(name: &str) -> Option<&'static str> {
    match resolve_model_name(name) {
        "emo" => Some("7ce76f9535cb254488985057c0d33006"),
        "v1_ctc" => Some("f027f199e590a391d015aeede2e66174"),
        "v1_rnnt" => Some("02c758999bcdc6afcb2087ef256d47ef"),
        "v2_ctc" => Some("e00f59cb5d39624fb30d1786044795bf"),
        "v2_rnnt" => Some("547460139acfebd842323f59ed54ab54"),
        "v3_ctc" => Some("73413e7be9c6a5935827bfab5c0dd678"),
        "v3_rnnt" => Some("0fd2c9a1ff66abd8d32a3a07f7592815"),
        "v3_e2e_ctc" => Some("367074d6498f426d960b25f49531cf68"),
        "v3_e2e_rnnt" => Some("2730de7545ac43ad256485a462b0a27a"),
        _ => None,
    }
}

/// Resolve short aliases (`rnnt` → `v2_rnnt`, …). Paths pass through unchanged by caller.
pub fn resolve_model_name(name: &str) -> &str {
    match name {
        "rnnt" => "v2_rnnt",
        "ctc" => "v2_ctc",
        "e2e_rnnt" => "v3_e2e_rnnt",
        "e2e_ctc" => "v3_e2e_ctc",
        other => other,
    }
}

pub fn is_catalog_name(name: &str) -> bool {
    let resolved = resolve_model_name(name);
    CATALOG.contains(&resolved)
}

pub fn needs_tokenizer(name: &str) -> bool {
    let n = resolve_model_name(name);
    n.contains("e2e") || n == "v1_rnnt"
}

/// Shown under `vd-giga install --help` and when MODEL is missing/unknown.
pub const INSTALL_HELP: &str = concat!(
    "Catalog models:\n",
    "  v3_e2e_rnnt    rnnt  e2e (punctuation)\n",
    "  v3_e2e_ctc     ctc   e2e (punctuation)\n",
    "  v3_rnnt        rnnt  ASR\n",
    "  v3_ctc         ctc   ASR\n",
    "  v2_rnnt        rnnt  ASR (default for -m)\n",
    "  v2_ctc         ctc   ASR\n",
    "  v1_rnnt        rnnt  ASR\n",
    "  v1_ctc         ctc   ASR\n",
    "\n",
    "Aliases: rnnt→v2_rnnt, ctc→v2_ctc, e2e_rnnt→v3_e2e_rnnt, e2e_ctc→v3_e2e_ctc\n",
    "Runtime today: CTC after convert. RNNT can be installed; inference TBD.\n",
    "\n",
    "Examples:\n",
    "  vd-giga install v3_e2e_ctc\n",
    "  vd-giga install ctc\n",
    "  vd-giga install --all\n",
    "  vd-giga list --all"
);


pub fn ckpt_url(name: &str) -> String {
    let n = resolve_model_name(name);
    format!("{CDN_BASE}/{n}.ckpt")
}

pub fn tokenizer_url(name: &str) -> String {
    let n = resolve_model_name(name);
    format!("{CDN_BASE}/{n}_tokenizer.model")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderKind {
    Ctc,
    Rnnt,
}

pub fn decoder_kind(name: &str) -> Option<DecoderKind> {
    let n = resolve_model_name(name);
    if !is_catalog_name(n) {
        return None;
    }
    if n.contains("ctc") {
        Some(DecoderKind::Ctc)
    } else {
        Some(DecoderKind::Rnnt)
    }
}

pub fn line_label(name: &str) -> String {
    let n = resolve_model_name(name);
    if n.contains("e2e") {
        if n.starts_with("v3") {
            "v3 e2e".into()
        } else {
            "e2e".into()
        }
    } else if n.starts_with("v3") {
        "v3".into()
    } else if n.starts_with("v2") {
        "v2".into()
    } else if n.starts_with("v1") {
        "v1".into()
    } else {
        "unknown".into()
    }
}
