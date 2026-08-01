//! Catalog of installable language packs.

use crate::types::Language;

#[derive(Debug, Clone, Copy)]
pub struct CatalogEntry {
    pub name: &'static str,
    pub language: Language,
    pub shipping: bool,
    pub version: u32,
    pub backend: &'static str,
}

pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        name: "ru",
        language: Language::Ru,
        shipping: true,
        version: 1,
        backend: "rules",
    },
    CatalogEntry {
        name: "en",
        language: Language::En,
        shipping: true,
        version: 1,
        backend: "rules",
    },
    CatalogEntry {
        name: "de",
        language: Language::De,
        shipping: false,
        version: 0,
        backend: "rules",
    },
];

pub fn resolve_model_name(name: &str) -> &str {
    match name {
        "ru" | "en" | "de" | "auto" => {
            if name == "auto" {
                "ru"
            } else {
                name
            }
        }
        other => other,
    }
}

pub fn is_catalog_name(name: &str) -> bool {
    let n = resolve_model_name(name);
    CATALOG.iter().any(|e| e.name == n)
}

pub fn entry(name: &str) -> Option<&'static CatalogEntry> {
    let n = resolve_model_name(name);
    CATALOG.iter().find(|e| e.name == n)
}

pub fn shipping_names() -> Vec<&'static str> {
    CATALOG
        .iter()
        .filter(|e| e.shipping)
        .map(|e| e.name)
        .collect()
}

pub fn catalog_help_lines() -> String {
    let mut s = String::from("Catalog packs:\n");
    for e in CATALOG {
        let mark = if e.shipping { "shipping" } else { "reserved" };
        s.push_str(&format!(
            "  {:<6}  {}  backend={}  ({})\n",
            e.name,
            e.language.as_str(),
            e.backend,
            mark
        ));
    }
    s.push_str(
        "\nExamples:\n  vd-fix-casing install ru\n  vd-fix-casing install en\n  vd-fix-casing install --all\n",
    );
    s
}
