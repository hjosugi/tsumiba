//! The shared secret between the app and its clients.
//!
//! The local API listens on loopback only, but any web page you visit can still
//! make your browser send requests to `127.0.0.1`. The token is what stops
//! those requests: it is never sent to a page, and the browser extension holds
//! it in its own storage.

use std::fs;
use std::path::Path;

use crate::error::{Error, IoContext, Result};
use crate::fsutil::write_private;

const TOKEN_BYTES: usize = 32;

/// Read the token, creating it on first use.
pub fn load_or_create(path: &Path) -> Result<String> {
    match fs::read_to_string(path) {
        Ok(text) if is_well_formed(text.trim()) => return Ok(text.trim().to_string()),
        Ok(_) => {} // empty or damaged: replace it
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(Error::Io {
                context: format!("read {}", path.display()),
                source: e,
            });
        }
    }
    regenerate(path)
}

/// Replace the token, which signs out every paired browser.
pub fn regenerate(path: &Path) -> Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context(|| format!("create {}", parent.display()))?;
    }
    let token = generate()?;
    write_private(path, token.as_bytes())?;
    Ok(token)
}

fn generate() -> Result<String> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| {
        Error::Invalid(format!("the operating system could not provide random bytes: {e}"))
    })?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn is_well_formed(token: &str) -> bool {
    token.len() == TOKEN_BYTES * 2 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Compare two secrets without leaking, through timing, how many leading bytes
/// matched.
pub fn matches(expected: &str, given: &str) -> bool {
    let (a, b) = (expected.as_bytes(), given.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_once_then_stable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        let first = load_or_create(&path).unwrap();
        let second = load_or_create(&path).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn damaged_token_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        fs::write(&path, "short").unwrap();
        let token = load_or_create(&path).unwrap();
        assert_eq!(token.len(), 64);
        assert_eq!(fs::read_to_string(&path).unwrap().trim(), token);
    }

    #[test]
    fn trailing_newline_is_tolerated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        let token = "ab".repeat(32);
        fs::write(&path, format!("{token}\n")).unwrap();
        assert_eq!(load_or_create(&path).unwrap(), token);
    }

    #[test]
    fn regenerate_changes_the_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        let first = load_or_create(&path).unwrap();
        let second = regenerate(&path).unwrap();
        assert_ne!(first, second);
        assert_eq!(load_or_create(&path).unwrap(), second);
    }

    #[test]
    fn comparison() {
        assert!(matches("abc", "abc"));
        assert!(!matches("abc", "abd"));
        assert!(!matches("abc", "ab"));
        assert!(!matches("", "x"));
        assert!(matches("", ""));
    }
}
