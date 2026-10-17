//! Wire types of the local HTTP API (`/v1`).
//!
//! The server (`tsumiba`), the command line client, the browser extension and
//! the documentation all describe this one contract. Fields only ever get added;
//! a client must ignore fields it does not know.

use serde::{Deserialize, Serialize};

/// Default loopback port. Chosen to sit in the dynamic range and to be easy to
/// remember; configurable in `config.toml`.
pub const DEFAULT_PORT: u16 = 47821;

pub const APP_NAME: &str = "tsumiba";

/// The file name of an uploaded file, percent-encoded as UTF-8.
pub const HEADER_NAME: &str = "X-Tsumiba-Name";
/// Where an uploaded file came from (usually a page URL), percent-encoded.
pub const HEADER_SOURCE: &str = "X-Tsumiba-Source";

/// `GET /v1/ping` — no authentication, so a client can tell tsumiba from some
/// other program that happens to hold the port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    pub app: String,
    pub version: String,
}

impl Ping {
    pub fn current() -> Ping {
        Ping {
            app: APP_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn is_tsumiba(&self) -> bool {
        self.app == APP_NAME
    }
}

/// `GET /v1/status`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub notes: usize,
    pub files: usize,
    /// Whether the shelf is on screen right now. Always `false` for `tsumiba serve`.
    pub visible: bool,
}

/// A note as the API shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteDto {
    pub id: String,
    /// First line of the note, without Markdown heading markers.
    pub title: String,
    pub body: String,
    pub pinned: bool,
    pub created_ms: u64,
    pub updated_ms: u64,
}

/// `POST /v1/notes`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NewNote {
    pub text: String,
    #[serde(default)]
    pub pinned: bool,
}

/// `PATCH /v1/notes/{id}` — absent fields are left alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NotePatch {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub pinned: Option<bool>,
}

/// A file on the shelf as the API shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDto {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub added_ms: u64,
    pub pinned: bool,
    /// Where it came from, when known (a page URL for browser uploads, the
    /// original path for local imports).
    pub source: Option<String>,
    /// `true` when the shelf only remembers the location instead of keeping a copy.
    pub link: bool,
    /// Absolute path of the stored file (or of the original, for links).
    pub path: String,
}

/// `POST /v1/files/import` — import files from paths on the server's machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ImportRequest {
    pub paths: Vec<String>,
    /// Store the files as links to the originals instead of copying, whatever
    /// the `storage.mode` setting says.
    #[serde(default)]
    pub link: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ImportResult {
    pub added: Vec<FileDto>,
    pub failed: Vec<ImportFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFailure {
    pub path: String,
    pub error: String,
}

/// `POST /v1/window`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowRequest {
    pub action: WindowAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowAction {
    Show,
    Hide,
    Toggle,
}

/// Body of every 4xx and 5xx response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub error: String,
}

/// Percent-encode `s` for use in an HTTP header value. Everything outside
/// `[A-Za-z0-9._~-]` becomes `%XX`, so header values stay printable ASCII
/// regardless of what the file name or URL contains.
pub fn encode_header(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'~' | b'-' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Inverse of [`encode_header`]. Malformed escapes are kept literally and
/// invalid UTF-8 is replaced, because a header must never make the server fail.
pub fn decode_header(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push(hi << 4 | lo);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_encoding_round_trips() {
        for s in [
            "plain.txt",
            "日本語 メモ (1).png",
            "a/b\\c?d=e&f#g",
            "100%",
            "%zz",
            "",
        ] {
            assert_eq!(decode_header(&encode_header(s)), s, "{s}");
        }
    }

    #[test]
    fn encoded_headers_are_printable_ascii() {
        let encoded = encode_header("日本語\n\"quoted\"");
        assert!(encoded.bytes().all(|b| b.is_ascii_graphic()), "{encoded}");
    }

    #[test]
    fn decoding_tolerates_garbage() {
        assert_eq!(decode_header("%"), "%");
        assert_eq!(decode_header("%4"), "%4");
        assert_eq!(decode_header("%zz"), "%zz");
        // Invalid UTF-8 becomes the replacement character instead of failing.
        assert_eq!(decode_header("%FF"), "\u{fffd}");
    }

    #[test]
    fn window_action_wire_format() {
        let json = serde_json::to_string(&WindowRequest {
            action: WindowAction::Toggle,
        })
        .unwrap();
        assert_eq!(json, r#"{"action":"toggle"}"#);
    }

    #[test]
    fn new_note_needs_only_text() {
        let note: NewNote = serde_json::from_str(r#"{"text":"hi"}"#).unwrap();
        assert_eq!(note.text, "hi");
        assert!(!note.pinned);
    }

    #[test]
    fn ping_identifies_tsumiba() {
        assert!(Ping::current().is_tsumiba());
        assert!(
            !Ping {
                app: "other".into(),
                version: "1".into()
            }
            .is_tsumiba()
        );
    }
}
