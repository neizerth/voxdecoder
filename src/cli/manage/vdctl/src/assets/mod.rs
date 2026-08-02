//! Asset management (delegates later).

use serde_json::json;

use crate::error::Error;
use crate::resolve::Platform;

pub fn list(_platform: &Platform, json: bool) -> Result<(), Error> {
    let value = json!({ "models": [], "assets": [] });
    crate::output::emit_value(json, value, |_| {
        println!("No assets indexed yet.");
    })
}

pub fn install(_name: &str) -> Result<(), Error> {
    Err(Error::NotImplemented(
        "vdctl assets install is not implemented yet".into(),
    ))
}

pub fn update(_name: Option<&str>) -> Result<(), Error> {
    Err(Error::NotImplemented(
        "vdctl assets update is not implemented yet".into(),
    ))
}

pub fn remove(_name: &str) -> Result<(), Error> {
    Err(Error::NotImplemented(
        "vdctl assets remove is not implemented yet".into(),
    ))
}
