//! MCP stdio server using newline-delimited JSON-RPC (MCP stdio transport).

pub mod protocol;
pub mod tools;

use std::io::{self, BufRead, Write};

use serde_json::Value;

use crate::client::RuntimeClient;

pub fn serve(client: RuntimeClient) -> Result<(), String> {
    if std::env::var_os("VD_MCP_TRACE").is_some() {
        let _ = writeln!(io::stderr(), "vd-mcp: serve starting (stdio)");
    }

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut writer = io::stdout();
    while let Some(message) = read_message(&mut reader)? {
        if std::env::var_os("VD_MCP_TRACE").is_some() {
            let _ = writeln!(
                io::stderr(),
                "vd-mcp: request {}",
                message
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("?")
            );
        }
        if let Some(response) = protocol::handle(&client, message) {
            write_message(&mut writer, &response)?;
        }
    }
    if std::env::var_os("VD_MCP_TRACE").is_some() {
        let _ = writeln!(io::stderr(), "vd-mcp: stdin closed");
    }
    Ok(())
}

/// Read one MCP stdio message (NDJSON). Also accepts legacy Content-Length frames.
fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut line = String::new();
    if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
        return Ok(None);
    }
    let trimmed = line.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return read_message(reader);
    }
    if let Some(value) = trimmed.strip_prefix("Content-Length:") {
        let length = value
            .trim()
            .parse::<usize>()
            .map_err(|e| format!("invalid Content-Length: {e}"))?;
        // Consume remaining header lines until the blank separator.
        loop {
            line.clear();
            if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                return Err("MCP Content-Length frame truncated in headers".into());
            }
            if line.trim_end_matches(['\r', '\n']).is_empty() {
                break;
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
        return serde_json::from_slice(&body)
            .map(Some)
            .map_err(|e| format!("invalid MCP JSON: {e}"));
    }
    serde_json::from_str(trimmed)
        .map(Some)
        .map_err(|e| format!("invalid MCP JSON: {e}"))
}

fn write_message(writer: &mut impl Write, message: &Value) -> Result<(), String> {
    let mut body = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    body.push(b'\n');
    writer.write_all(&body).map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())
}

#[cfg(test)]
mod framing_tests {
    use super::{read_message, write_message};
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn reads_ndjson() {
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":0,\"method\":\"initialize\"}\n";
        let mut cur = Cursor::new(&input[..]);
        let msg = read_message(&mut cur).unwrap().unwrap();
        assert_eq!(msg["method"], "initialize");
        assert_eq!(msg["id"], 0);
    }

    #[test]
    fn reads_content_length_legacy() {
        let body = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
        let mut frame = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        frame.extend_from_slice(body);
        let mut cur = Cursor::new(frame);
        let msg = read_message(&mut cur).unwrap().unwrap();
        assert_eq!(msg["method"], "ping");
    }

    #[test]
    fn writes_ndjson_with_trailing_newline() {
        let mut out = Vec::new();
        write_message(&mut out, &json!({"ok": true})).unwrap();
        assert_eq!(out, b"{\"ok\":true}\n");
    }
}
