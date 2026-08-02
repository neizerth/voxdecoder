//! Human vs machine output.

use serde::Serialize;
use serde_json::Value;

use crate::error::Error;

pub fn emit_json(value: &impl Serialize) -> Result<(), Error> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|e| Error::Message(e.to_string()))?
    );
    Ok(())
}

pub fn emit_value(json: bool, value: Value, human: impl FnOnce(&Value)) -> Result<(), Error> {
    if json {
        emit_json(&value)
    } else {
        human(&value);
        Ok(())
    }
}
