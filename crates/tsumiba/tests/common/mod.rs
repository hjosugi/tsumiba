//! A minimal raw HTTP/1.1 client for the integration tests.
//!
//! Hand-written on purpose: the tests need to send requests a well-behaved
//! client library would refuse to (a forged `Host`, an `Origin` header, a
//! `Content-Length` that lies), and to see every header that comes back.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("not JSON ({e}): {:?}", self.text()))
    }
}

pub struct Request<'a> {
    pub method: &'a str,
    pub path: &'a str,
    pub headers: Vec<(&'a str, String)>,
    pub body: &'a [u8],
    /// Override the Host header (defaults to the real address).
    pub host: Option<&'a str>,
    /// Claim a different Content-Length than the body's.
    pub claimed_length: Option<usize>,
    /// Send the body with `Transfer-Encoding: chunked` and no Content-Length,
    /// so the server cannot know the size in advance.
    pub chunked: bool,
}

impl<'a> Request<'a> {
    pub fn new(method: &'a str, path: &'a str) -> Self {
        Request {
            method,
            path,
            headers: Vec::new(),
            body: &[],
            host: None,
            claimed_length: None,
            chunked: false,
        }
    }

    pub fn header(mut self, name: &'a str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    pub fn bearer(self, token: &str) -> Self {
        self.header("Authorization", format!("Bearer {token}"))
    }

    pub fn body(mut self, body: &'a [u8]) -> Self {
        self.body = body;
        self
    }

    pub fn json(self, body: &'a str) -> Self {
        self.header("Content-Type", "application/json")
            .body(body.as_bytes())
    }

    pub fn host(mut self, host: &'a str) -> Self {
        self.host = Some(host);
        self
    }

    pub fn claimed_length(mut self, n: usize) -> Self {
        self.claimed_length = Some(n);
        self
    }

    pub fn chunked(mut self) -> Self {
        self.chunked = true;
        self
    }

    /// Send the request and return the response, or `None` if the server closed
    /// the connection without answering (which it may do to a rude client).
    pub fn try_send(self, port: u16) -> Option<Response> {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();

        let host = self
            .host
            .map(str::to_string)
            .unwrap_or_else(|| format!("127.0.0.1:{port}"));
        let length = self.claimed_length.unwrap_or(self.body.len());
        let framing = if self.chunked {
            "Transfer-Encoding: chunked".to_string()
        } else {
            format!("Content-Length: {length}")
        };
        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n{framing}\r\n",
            self.method, self.path
        );
        for (name, value) in &self.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");

        // Read the response on another thread *while* the body is still going out.
        // A server that answers early (an oversized upload, say) closes the socket
        // with our bytes unread, and Windows and macOS then reset the connection,
        // throwing away a response we have not read yet. Reading as it arrives
        // keeps it.
        let mut reader = stream.try_clone().expect("clone the socket");
        let collector = std::thread::spawn(move || {
            let mut raw = Vec::new();
            let mut buffer = [0u8; 8192];
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buffer[..n]);
            }
            raw
        });

        // The server may stop listening before the whole body is sent, as it
        // should for an oversized upload; a failed write is not a test failure.
        let _ = stream.write_all(head.as_bytes());
        if self.chunked {
            for chunk in self.body.chunks(64 * 1024) {
                let _ = stream.write_all(format!("{:x}\r\n", chunk.len()).as_bytes());
                let _ = stream.write_all(chunk);
                let _ = stream.write_all(b"\r\n");
            }
            let _ = stream.write_all(b"0\r\n\r\n");
        } else {
            let _ = stream.write_all(self.body);
        }
        let _ = stream.flush();

        let raw = collector.join().expect("the reader thread");
        if raw.windows(4).any(|w| w == b"\r\n\r\n") {
            Some(parse(&raw))
        } else {
            None
        }
    }

    pub fn send(self, port: u16) -> Response {
        self.try_send(port)
            .expect("the server closed the connection without answering")
    }
}

fn parse(raw: &[u8]) -> Response {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("no header terminator in {:?}", String::from_utf8_lossy(raw)));
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.split("\r\n");
    let status_line = lines.next().expect("status line");
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("bad status line {status_line:?}"));
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect::<Vec<_>>();
    let mut body = raw[split + 4..].to_vec();

    // De-chunk if needed.
    if headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("transfer-encoding") && v.eq_ignore_ascii_case("chunked")
    }) {
        body = dechunk(&body);
    }
    Response {
        status,
        headers,
        body,
    }
}

fn dechunk(mut data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(line_end) = data.windows(2).position(|w| w == b"\r\n") {
        let size = usize::from_str_radix(
            String::from_utf8_lossy(&data[..line_end])
                .split(';')
                .next()
                .unwrap()
                .trim(),
            16,
        )
        .unwrap_or(0);
        data = &data[line_end + 2..];
        if size == 0 || data.len() < size {
            break;
        }
        out.extend_from_slice(&data[..size]);
        data = &data[(size + 2).min(data.len())..];
    }
    out
}
