//! Windows named pipe transport (stub).
//!
//! Full Named Pipe support lands with the Client SDK work; until then Windows
//! defaults to TCP (`TransportKind::Auto` → `Tcp` on this platform).

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::engine::Engine;

use super::Duplex;

pub fn serve(_name: &str, _engine: Engine, _stop: Arc<AtomicBool>) -> Result<(), String> {
    Err(
        "named pipe transport is not implemented yet; use --transport tcp \
         (e.g. --tcp 127.0.0.1:7701)"
            .into(),
    )
}

pub fn connect(_name: &str) -> Result<Duplex, String> {
    Err(
        "named pipe transport is not implemented yet; use --transport tcp \
         (e.g. --tcp 127.0.0.1:7701)"
            .into(),
    )
}
