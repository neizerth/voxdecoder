//! Media providers (YouTube · direct · stub).

mod direct;
mod stub;
mod tools;
mod youtube;

use crate::import::{ImportError, ImportResult, ProviderId, UrlImportRequest};

pub use tools::{doctor_report, DoctorCheck};

pub trait MediaProvider {
    fn id(&self) -> ProviderId;
    fn supports_subtitles(&self) -> bool;
    fn resolve(&self, request: &UrlImportRequest) -> Result<ImportResult, ImportError>;
}

pub fn resolve_provider(id: ProviderId) -> Result<Box<dyn MediaProvider>, ImportError> {
    match id {
        ProviderId::Youtube => Ok(Box::new(youtube::YoutubeProvider)),
        ProviderId::Direct => Ok(Box::new(direct::DirectProvider)),
        ProviderId::Stub => Ok(Box::new(stub::StubProvider)),
    }
}

/// Human-readable capability catalog for `vd-url providers`.
pub fn catalog_lines() -> Vec<String> {
    vec![
        "youtube".into(),
        "  audio".into(),
        "  metadata".into(),
        "  subtitles".into(),
        "  inspect".into(),
        String::new(),
        "direct".into(),
        "  audio".into(),
        "  metadata".into(),
        String::new(),
        "stub".into(),
        "  audio".into(),
        "  metadata".into(),
        "  subtitles".into(),
        "  inspect".into(),
    ]
}
