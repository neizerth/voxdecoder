//! Optional TCP transport.

use std::io::{BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::engine::Engine;

use super::{run_session, Duplex};

pub fn serve(addr: SocketAddr, engine: Engine, stop: Arc<AtomicBool>) -> Result<(), String> {
    let listener = TcpListener::bind(addr).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;

    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                // Same as UDS: clear inherited O_NONBLOCK on macOS/BSD.
                if let Err(e) = stream.set_nonblocking(false) {
                    eprintln!("vd-srv: accepted tcp set_nonblocking(false): {e}");
                    continue;
                }
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
    Ok(())
}

pub fn connect(addr: SocketAddr) -> Result<Duplex, String> {
    let stream = TcpStream::connect(addr)
        .map_err(|e| format!("cannot connect to {addr}: {e} (is vd-srv serve running?)"))?;
    let writer = stream.try_clone().map_err(|e| e.to_string())?;
    Ok(Duplex {
        reader: BufReader::new(Box::new(stream) as Box<dyn std::io::Read + Send>),
        writer: Box::new(writer) as Box<dyn Write + Send>,
    })
}
