//! Time-ordered identifiers.
//!
//! An id is 20 lowercase hex digits: the creation time in milliseconds since the
//! Unix epoch (12 digits) followed by 32 random bits (8 digits). Sorting ids as
//! strings therefore sorts by creation time, and the creation time can be read
//! back from the id, so it does not need to be stored anywhere else.
//!
//! Ids become file and directory names, and they arrive over HTTP, so
//! [`is_valid`] is the gate that keeps path separators and `..` out of the
//! filesystem.

use std::time::{SystemTime, UNIX_EPOCH};

pub const ID_LEN: usize = 20;

/// Milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// A fresh id for "now".
pub fn new_id() -> String {
    from_parts(now_ms(), random_u32())
}

/// Build an id from its two halves. Exposed so tests can be deterministic.
pub fn from_parts(ms: u64, random: u32) -> String {
    // 12 hex digits hold 48 bits, which is good until the year 10889.
    format!("{:012x}{:08x}", ms & 0xffff_ffff_ffff, random)
}

/// Whether `s` has the exact shape of an id.
pub fn is_valid(s: &str) -> bool {
    s.len() == ID_LEN && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The creation time embedded in an id, if the id is valid.
pub fn created_ms(id: &str) -> Option<u64> {
    if !is_valid(id) {
        return None;
    }
    u64::from_str_radix(&id[..12], 16).ok()
}

fn random_u32() -> u32 {
    // The operating system's generator can fail only in exotic sandboxes. An id
    // does not need to be unpredictable, only unlikely to collide within one
    // millisecond, so fall back to the clock's sub-millisecond part.
    getrandom::u32().unwrap_or_else(|_| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ids_are_valid_and_unique() {
        let a = new_id();
        let b = new_id();
        assert!(is_valid(&a));
        assert!(is_valid(&b));
        assert_ne!(a, b);
    }

    #[test]
    fn ids_sort_by_creation_time() {
        let earlier = from_parts(1_000, 0xffff_ffff);
        let later = from_parts(1_001, 0);
        assert!(earlier < later);
    }

    #[test]
    fn created_time_round_trips() {
        let id = from_parts(1_760_000_000_123, 42);
        assert_eq!(created_ms(&id), Some(1_760_000_000_123));
    }

    #[test]
    fn rejects_anything_that_is_not_an_id() {
        assert!(!is_valid(""));
        assert!(!is_valid("../../etc/passwd"));
        assert!(!is_valid("0000000000000000000")); // 19 digits
        assert!(!is_valid("000000000000000000000")); // 21 digits
        assert!(!is_valid("0000000000000000000G"));
        assert!(!is_valid("0000000000000000000A")); // uppercase
        assert!(!is_valid("00000000000000000/00"));
        assert_eq!(created_ms("nope"), None);
    }
}
