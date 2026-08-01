//! Shipping lexicon for common tech terminology (`--language ru` focus).

use crate::types::Language;

use super::TermEntry;

/// Built-in entries. Shared Latin tech names; Cyrillic ASR-ish variants for `ru`.
pub fn entries(language: Language) -> Vec<TermEntry> {
    match language {
        Language::Ru | Language::Auto | Language::En | Language::De => ru_tech(),
    }
}

fn ru_tech() -> Vec<TermEntry> {
    vec![
        entry("Kubernetes", &["k8s", "кубернетис", "кубернетес"]),
        entry(
            "GitHub Actions",
            &[
                "github actions",
                "гитхаб экшенс",
                "гитхап экшенс",
                "github action",
            ],
        ),
        entry("GitHub", &["github", "гитхаб", "гитхап"]),
        entry("JSON", &["json", "джи сон", "джейсон"]),
        entry(
            "PostgreSQL",
            &["postgres", "postgresql", "постгрес", "постгрескьюэль"],
        ),
        entry("Docker", &["docker", "докер"]),
        entry("TypeScript", &["typescript", "тайпскрипт"]),
        entry("JavaScript", &["javascript", "джаваскрипт"]),
        entry("GraphQL", &["graphql", "графкьюэль"]),
        entry("API", &["апи"]),
        entry("HTTP", &["ашттп"]),
        entry("HTTPS", &["https"]),
        entry("gRPC", &["grpc", "джи ар пи си"]),
        entry("Redis", &["redis", "редис"]),
        entry("Kafka", &["kafka", "кафка"]),
        entry("Nginx", &["nginx", "энжинкс", "нджинкс"]),
        entry("Linux", &["linux", "линукс"]),
        entry("macOS", &["macos", "мак ос"]),
        entry("Python", &["python", "питон"]),
        entry("Rust", &["раст"]),
        entry("YAML", &["yaml", "ямл"]),
        entry("TOML", &["toml"]),
        entry("SQL", &["эскьюэль"]),
        entry("HTML", &["аштиемель"]),
        entry("WebSocket", &["websocket", "вебсокет", "web socket"]),
        entry("OpenAPI", &["openapi", "open api"]),
        entry("OAuth", &["oauth", "оаутх"]),
        entry("JWT", &["jwt"]),
        entry("CI/CD", &["ci/cd", "cicd", "сиайсиди"]),
    ]
}

fn entry(canonical: &str, variants: &[&str]) -> TermEntry {
    TermEntry {
        canonical: canonical.to_string(),
        variants: variants.iter().map(|s| (*s).to_string()).collect(),
    }
}
