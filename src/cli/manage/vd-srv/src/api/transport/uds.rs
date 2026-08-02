//! Unix domain socket transport.

use std::io::{BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::engine::Engine;

use super::{run_session, Duplex};

pub fn serve(socket: &Path, engine: Engine, stop: Arc<AtomicBool>) -> Result<(), String> {
    if socket.exists() {
        std::fs::remove_file(socket).map_err(|e| e.to_string())?;
    }
    if let Some(parent) = socket.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let listener = UnixListener::bind(socket).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;

    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let eng = engine.clone();
                thread::spawn(move || {
                    if let Ok(writer) = stream.try_clone() {
                        let reader = BufReader::new(stream);
                        let _ = run_session(reader, writer, &eng);
                    }
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let _ = std::fs::remove_file(socket);
    Ok(())
}

pub fn connect(socket: &Path) -> Result<Duplex, String> {
    let stream = UnixStream::connect(socket).map_err(|e| {
        format!(
            "cannot connect to {}: {e} (is vd-srv serve running?)",
            socket.display()
        )
    })?;
    let writer = stream.try_clone().map_err(|e| e.to_string())?;
    Ok(Duplex {
        reader: BufReader::new(Box::new(stream) as Box<dyn std::io::Read + Send>),
        writer: Box::new(writer) as Box<dyn Write + Send>,
    })
}
