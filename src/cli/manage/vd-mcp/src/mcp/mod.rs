//! MCP stdio server using Content-Length framed JSON-RPC.

pub mod protocol;
pub mod tools;

use std::io::{self, BufRead, Write};

use serde_json::Value;

use crate::client::RuntimeClient;

pub fn serve(client: RuntimeClient) -> Result<(), String> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    while let Some(message) = read_frame(&mut reader)? {
        if let Some(response) = protocol::handle(&client, message) {
            write_frame(&mut writer, &response)?;
        }
    }
    Ok(())
}

fn read_frame(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| format!("invalid Content-Length: {e}"))?,
            );
        }
    }
    let length = content_length.ok_or_else(|| "MCP frame missing Content-Length".to_string())?;
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|e| format!("invalid MCP JSON: {e}"))
}

fn write_frame(writer: &mut impl Write, message: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
    writer.write_all(&body).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}
