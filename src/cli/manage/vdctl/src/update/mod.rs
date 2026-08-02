//! Platform install / update / uninstall (ADR 0003).

use crate::error::Error;
use crate::resolve::{self, Platform};

pub fn install(platform: &Platform) -> Result<(), Error> {
    resolve::refuse_release_ops(platform)?;
    Err(Error::NotImplemented(
        "vdctl install is not implemented yet (GitHub Releases)".into(),
    ))
}

pub fn update(platform: &Platform, _channel: Option<&str>) -> Result<(), Error> {
    resolve::refuse_release_ops(platform)?;
    Err(Error::NotImplemented(
        "vdctl update is not implemented yet (GitHub Releases)".into(),
    ))
}

pub fn uninstall(platform: &Platform, _purge: bool) -> Result<(), Error> {
    resolve::refuse_release_ops(platform)?;
    Err(Error::NotImplemented(
        "vdctl uninstall is not implemented yet".into(),
    ))
}
