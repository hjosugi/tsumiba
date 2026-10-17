//! Small filesystem helpers shared by the store, the config and the token.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::error::{IoContext, Result};
use crate::id;

/// Longest file name, in bytes, that tsumiba will create. Most filesystems cap a
/// single component at 255 bytes; leave room for a suffix.
const MAX_NAME_BYTES: usize = 200;

/// Replace `path` with `bytes` so that a reader sees either the old content or
/// the new content, never a half-written file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = temp_sibling(path);
    {
        let mut file = File::create(&tmp).context(|| format!("create {}", tmp.display()))?;
        file.write_all(bytes)
            .context(|| format!("write {}", tmp.display()))?;
        file.sync_all()
            .context(|| format!("flush {}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        crate::error::Error::Io {
            context: format!("replace {}", path.display()),
            source: e,
        }
    })
}

/// Like [`write_atomic`], and the file is readable by its owner only on Unix.
/// Used for the API token.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let tmp = temp_sibling(path);
    {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&tmp)
            .context(|| format!("create {}", tmp.display()))?;
        file.write_all(bytes)
            .context(|| format!("write {}", tmp.display()))?;
        file.sync_all()
            .context(|| format!("flush {}", tmp.display()))?;
    }
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        crate::error::Error::Io {
            context: format!("replace {}", path.display()),
            source: e,
        }
    })
}

/// A path next to `path` that no other writer will pick.
pub fn temp_sibling(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(format!(".tmp-{}-{}", std::process::id(), id::new_id()));
    path.with_file_name(name)
}

/// Turn an arbitrary string into something every supported filesystem accepts
/// as one path component.
///
/// - path separators, control characters and `<>:"|?*` become `_`
/// - leading and trailing spaces and dots are dropped (Windows rejects them)
/// - names Windows reserves (`CON`, `NUL`, `COM1`, ...) get a leading `_`
/// - the result is at most 200 bytes, keeping the extension when there is one
/// - an empty result becomes `file`
pub fn sanitize_file_name(name: &str) -> String {
    // A caller may hand over a whole path ("C:\\Users\\me\\a.txt"). Keep only
    // the last component.
    let last = name.rsplit(['/', '\\']).next().unwrap_or(name);

    let mut cleaned: String = last
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '/' | '\\') {
                '_'
            } else {
                c
            }
        })
        .collect();

    cleaned = cleaned.trim_matches([' ', '.']).to_string();
    if cleaned.is_empty() {
        return "file".to_string();
    }

    if is_windows_reserved(&cleaned) {
        cleaned.insert(0, '_');
    }

    truncate_keeping_extension(&cleaned, MAX_NAME_BYTES)
}

fn is_windows_reserved(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn truncate_keeping_extension(name: &str, max: usize) -> String {
    if name.len() <= max {
        return name.to_string();
    }
    let (stem, ext) = match name.rfind('.') {
        // A dot at index 0 is a dotfile, not an extension; a very long
        // "extension" is not one either.
        Some(i) if i > 0 && name.len() - i <= 16 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let budget = max.saturating_sub(ext.len());
    let mut end = budget.min(stem.len());
    while !stem.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &stem[..end], ext)
}

/// Copy `src` to `dst`, creating `dst` exclusively. Returns the byte count.
pub fn copy_new(src: &Path, dst: &Path) -> Result<u64> {
    let mut input = File::open(src).context(|| format!("open {}", src.display()))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
        .context(|| format!("create {}", dst.display()))?;
    let n = io::copy(&mut input, &mut output).context(|| format!("copy {}", src.display()))?;
    output
        .sync_all()
        .context(|| format!("flush {}", dst.display()))?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_pass_through() {
        assert_eq!(sanitize_file_name("report.pdf"), "report.pdf");
        assert_eq!(sanitize_file_name("日本語 メモ.txt"), "日本語 メモ.txt");
    }

    #[test]
    fn paths_are_reduced_to_their_last_component() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_file_name("C:\\Users\\me\\a.txt"), "a.txt");
        assert_eq!(sanitize_file_name("dir/"), "file");
    }

    #[test]
    fn forbidden_characters_are_replaced() {
        assert_eq!(
            sanitize_file_name("a<b>c:d\"e|f?g*h.txt"),
            "a_b_c_d_e_f_g_h.txt"
        );
        assert_eq!(sanitize_file_name("tab\there"), "tab_here");
    }

    #[test]
    fn leading_and_trailing_dots_and_spaces_go() {
        assert_eq!(sanitize_file_name("  .hidden. "), "hidden");
        assert_eq!(sanitize_file_name("..."), "file");
        assert_eq!(sanitize_file_name(""), "file");
    }

    #[test]
    fn reserved_windows_names_are_prefixed() {
        assert_eq!(sanitize_file_name("CON"), "_CON");
        assert_eq!(sanitize_file_name("nul.txt"), "_nul.txt");
        assert_eq!(sanitize_file_name("com1.log"), "_com1.log");
        assert_eq!(sanitize_file_name("LPT9"), "_LPT9");
        assert_eq!(sanitize_file_name("COM10"), "COM10");
        assert_eq!(sanitize_file_name("console.txt"), "console.txt");
    }

    #[test]
    fn long_names_keep_their_extension_and_stay_valid_utf8() {
        let long = format!("{}.tar.gz", "あ".repeat(200));
        let out = sanitize_file_name(&long);
        assert!(out.len() <= MAX_NAME_BYTES, "{} bytes", out.len());
        assert!(out.ends_with(".gz"));
        // Still a valid String, so char boundaries were respected.
        assert!(out.chars().count() > 10);
    }

    #[test]
    fn atomic_write_replaces_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.json");
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        // No temporary files are left behind.
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(leftovers.len(), 1, "{leftovers:?}");
    }

    #[cfg(unix)]
    #[test]
    fn private_files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret");
        write_private(&path, b"x").unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn copy_new_refuses_to_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        fs::write(&src, b"hello").unwrap();
        assert_eq!(copy_new(&src, &dst).unwrap(), 5);
        assert!(copy_new(&src, &dst).is_err());
    }
}
