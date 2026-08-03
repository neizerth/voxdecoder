//! Transport abstraction — bytes in/out; JSON-RPC sits above.

#[cfg(windows)]
mod pipe;
mod tcp;
#[cfg(unix)]
mod uds;

use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::engine::Engine;

use super::dispatch;
use super::rpc::{ErrorObject, Outbound, Request, Response};

/// Selected transport kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    /// Platform default: UDS on Unix, Named Pipe on Windows.
    #[default]
    Auto,
    /// Unix domain socket.
    Uds,
    /// Windows named pipe.
    Pipe,
    /// Optional TCP (disabled by default at the config layer).
    Tcp,
}

impl TransportKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "uds" | "unix" | "socket" => Some(Self::Uds),
            "pipe" | "named_pipe" | "named-pipe" => Some(Self::Pipe),
            "tcp" => Some(Self::Tcp),
            _ => None,
        }
    }

    pub fn resolve(self) -> ResolvedKind {
        match self {
            Self::Auto => {
                #[cfg(unix)]
                {
                    ResolvedKind::Uds
                }
                // Named Pipe primary on Windows once pipe transport ships; TCP until then.
                #[cfg(windows)]
                {
                    ResolvedKind::Tcp
                }
                #[cfg(not(any(unix, windows)))]
                {
                    ResolvedKind::Tcp
                }
            }
            Self::Uds => ResolvedKind::Uds,
            Self::Pipe => ResolvedKind::Pipe,
            Self::Tcp => ResolvedKind::Tcp,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedKind {
    Uds,
    Pipe,
    Tcp,
}

/// Where to listen / connect.
#[derive(Debug, Clone)]
pub enum Endpoint {
    /// Filesystem path for a Unix domain socket.
    Uds(PathBuf),
    /// Named pipe path (`\\.\pipe\vd-srv` on Windows).
    Pipe(String),
    /// TCP socket address.
    Tcp(SocketAddr),
}

impl Endpoint {
    pub fn display(&self) -> String {
        match self {
            Self::Uds(p) => p.display().to_string(),
            Self::Pipe(n) => n.clone(),
            Self::Tcp(a) => a.to_string(),
        }
    }
}

/// Build the primary endpoint from config + CLI overrides.
pub fn resolve_endpoint(
    kind: TransportKind,
    socket: Option<&Path>,
    pipe: Option<&str>,
    tcp: Option<&str>,
    data_dir: &Path,
) -> Result<Endpoint, String> {
    match kind.resolve() {
        ResolvedKind::Uds => {
            #[cfg(unix)]
            {
                let path = socket
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| data_dir.join("vd-srv.sock"));
                Ok(Endpoint::Uds(path))
            }
            #[cfg(not(unix))]
            {
                let _ = (socket, data_dir);
                Err("uds transport is only available on Unix".into())
            }
        }
        ResolvedKind::Pipe => {
            #[cfg(windows)]
            {
                let name = pipe
                    .map(str::to_string)
                    .unwrap_or_else(|| r"\\.\pipe\vd-srv".into());
                Ok(Endpoint::Pipe(name))
            }
            #[cfg(not(windows))]
            {
                let _ = pipe;
                Err("pipe transport is only available on Windows".into())
            }
        }
        ResolvedKind::Tcp => {
            let addr = tcp.unwrap_or("127.0.0.1:7701");
            addr.parse::<SocketAddr>()
                .map(Endpoint::Tcp)
                .map_err(|e| format!("invalid tcp address {addr}: {e}"))
        }
    }
}

/// Accept loop for one endpoint; each connection speaks NDJSON JSON-RPC.
pub fn serve(endpoint: &Endpoint, engine: Engine, stop: Arc<AtomicBool>) -> Result<(), String> {
    match endpoint {
        #[cfg(unix)]
        Endpoint::Uds(path) => uds::serve(path, engine, stop),
        #[cfg(not(unix))]
        Endpoint::Uds(_) => Err("uds not supported on this platform".into()),

        #[cfg(windows)]
        Endpoint::Pipe(name) => pipe::serve(name, engine, stop),
        #[cfg(not(windows))]
        Endpoint::Pipe(_) => Err("named pipe not supported on this platform".into()),

        Endpoint::Tcp(addr) => tcp::serve(*addr, engine, stop),
    }
}

/// Open a duplex connection to the endpoint.
pub fn connect(endpoint: &Endpoint) -> Result<Duplex, String> {
    match endpoint {
        #[cfg(unix)]
        Endpoint::Uds(path) => uds::connect(path),
        #[cfg(not(unix))]
        Endpoint::Uds(_) => Err("uds not supported on this platform".into()),

        #[cfg(windows)]
        Endpoint::Pipe(name) => pipe::connect(name),
        #[cfg(not(windows))]
        Endpoint::Pipe(_) => Err("named pipe not supported on this platform".into()),

        Endpoint::Tcp(addr) => tcp::connect(*addr),
    }
}

/// Read + write halves of a transport connection.
pub struct Duplex {
    pub reader: BufReader<Box<dyn std::io::Read + Send>>,
    pub writer: Box<dyn Write + Send>,
}

pub(crate) fn run_session(
    mut reader: impl BufRead,
    mut writer: impl Write,
    engine: &Engine,
) -> Result<(), String> {
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let outbound = match serde_json::from_str::<Request>(trimmed) {
            Ok(req) => match dispatch::handle(engine, req) {
                Some(resp) => Outbound::Response(resp),
                None => continue,
            },
            Err(e) => Outbound::Response(Response::failure(
                None,
                ErrorObject::parse(format!("invalid JSON-RPC: {e}")),
            )),
        };
        write_frame(&mut writer, &outbound)?;
    }
    Ok(())
}

pub(crate) fn write_frame(writer: &mut impl Write, msg: &Outbound) -> Result<(), String> {
    let mut out = serde_json::to_vec(msg).map_err(|e| e.to_string())?;
    out.push(b'\n');
    writer.write_all(&out).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}
