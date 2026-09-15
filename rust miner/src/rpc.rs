//! Minimal JSON-RPC client for bitcoind, over a raw TCP socket.
//!
//! Deliberately no HTTP library: a request is a few lines of text plus a JSON
//! body — exactly what `curl -v` showed. A raw socket also ignores the
//! `http_proxy` environment variable, which otherwise routes localhost traffic
//! through the proxy and comes back as `502 Bad Gateway`.

use serde_json::{json, Value};
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub struct Rpc {
    addr: String,
    cookie_path: String,
}

impl Rpc {
    pub fn new(addr: &str, cookie_path: &str) -> Rpc {
        Rpc {
            addr: addr.to_string(),
            cookie_path: cookie_path.to_string(),
        }
    }

    /// Call one RPC method; return its `result`, or the node's error as `Err`.
    pub fn call(&self, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
        // bitcoind writes a fresh cookie on every start, so read it per call.
        let cookie = std::fs::read_to_string(&self.cookie_path)?;
        let body = json!({"jsonrpc": "1.0", "id": "miner", "method": method, "params": params})
            .to_string();

        let request = format!(
            "POST / HTTP/1.1\r\n\
             Host: {}\r\n\
             Authorization: Basic {}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            self.addr,
            base64(cookie.trim().as_bytes()),
            body.len(),
            body
        );
        let mut stream = TcpStream::connect(&self.addr)?;
        stream.write_all(request.as_bytes())?;

        // Status line, then headers up to a blank line, then exactly
        // Content-Length bytes of body.
        let mut reader = BufReader::new(stream);
        let mut status = String::new();
        reader.read_line(&mut status)?;
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse()?;
                }
            }
        }
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body)?;

        // A 401 (bad cookie) has no body; RPC errors come back as JSON.
        if body.is_empty() {
            return Err(format!("{method}: {} (empty body; stale cookie?)", status.trim()).into());
        }
        let reply: Value = serde_json::from_slice(&body)?;
        if !reply["error"].is_null() {
            return Err(format!("{method}: {}", reply["error"]).into());
        }
        Ok(reply["result"].clone())
    }
}

/// Standard base64, for the HTTP Basic `Authorization` header.
fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        // Pack up to 3 bytes into 24 bits, then emit 6 bits per character.
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = (chunk[0] as u32) << 16 | (b1 as u32) << 8 | b2 as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_padding() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"__cookie__"), "X19jb29raWVfXw==");
    }
}
