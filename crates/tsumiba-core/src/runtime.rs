//! The record a running instance leaves so the command line can find it.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, IoContext, Result};
use crate::fsutil::write_atomic;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub port: u16,
    pub pid: u32,
    pub version: String,
}

impl ServerInfo {
    pub fn write(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self).map_err(|e| Error::Index(e.to_string()))?;
        write_atomic(path, &json)
    }

    /// The record, if there is one and it is readable. A stale record (the
    /// process is gone) still reads fine; callers confirm with a ping.
    pub fn read(path: &Path) -> Option<ServerInfo> {
        let bytes = fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn remove(path: &Path) -> Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).context(|| format!("remove {}", path.display())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_remove() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.json");
        assert_eq!(ServerInfo::read(&path), None);
        let info = ServerInfo {
            port: 47821,
            pid: 7,
            version: "0.1.0".into(),
        };
        info.write(&path).unwrap();
        assert_eq!(ServerInfo::read(&path), Some(info));
        ServerInfo::remove(&path).unwrap();
        ServerInfo::remove(&path).unwrap(); // idempotent
        assert_eq!(ServerInfo::read(&path), None);
    }

    #[test]
    fn garbage_reads_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.json");
        fs::write(&path, "not json").unwrap();
        assert_eq!(ServerInfo::read(&path), None);
    }
}
