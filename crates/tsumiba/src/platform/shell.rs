//! Handing files to the rest of the desktop: open, reveal, clipboard.

use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// Open `path` with whatever the system associates with it.
pub fn open(path: &Path) -> io::Result<()> {
    open::that_detached(path)
}

/// Show `path` selected in the system file manager.
pub fn reveal(path: &Path) -> io::Result<()> {
    imp::reveal(path)
}

/// Put the file itself (not its path) on the clipboard, so it can be pasted
/// into a file manager, a mail composer or a chat window.
pub fn copy_file_to_clipboard(path: &Path) -> Result<(), String> {
    use clipboard_rs::{Clipboard, ClipboardContext};
    let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
    let path = path.to_str().ok_or("the path is not valid UTF-8")?;
    ctx.set_files(vec![path.to_string()])
        .map_err(|e| e.to_string())
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn spawn_quiet(mut command: Command) -> io::Result<()> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::os::windows::process::CommandExt;

    pub fn reveal(path: &Path) -> io::Result<()> {
        // explorer.exe wants `/select,"C:\path"` as one raw argument; the usual
        // argument quoting would wrap the whole thing in quotes and it would
        // ignore it. (It also exits non-zero on success, so only spawning is checked.)
        let mut command = Command::new("explorer.exe");
        command.raw_arg(format!("/select,\"{}\"", path.display()));
        spawn_quiet(command)
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    pub fn reveal(path: &Path) -> io::Result<()> {
        let mut command = Command::new("open");
        command.arg("-R").arg(path);
        spawn_quiet(command)
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod imp {
    use super::*;

    pub fn reveal(path: &Path) -> io::Result<()> {
        // The freedesktop FileManager1 interface selects the file in the user's
        // file manager. If nothing answers, open the folder instead.
        let uri = format!("file://{}", percent_encode_path(&path.to_string_lossy()));
        let mut dbus = Command::new("dbus-send");
        dbus.args([
            "--session",
            "--print-reply",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{uri}"))
        .arg("string:");
        dbus.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(status) = dbus.status()
            && status.success()
        {
            return Ok(());
        }
        let folder = path.parent().unwrap_or(path);
        open::that_detached(folder)
    }

    /// Percent-encode everything a `file://` URI does not allow, keeping `/`.
    pub(super) fn percent_encode_path(path: &str) -> String {
        let mut out = String::with_capacity(path.len());
        for b in path.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char)
                }
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    }
}

#[cfg(all(test, not(any(target_os = "windows", target_os = "macos"))))]
mod tests {
    use super::imp::percent_encode_path;

    #[test]
    fn uri_encoding_keeps_slashes_and_escapes_the_rest() {
        assert_eq!(
            percent_encode_path("/home/a b/日本.txt"),
            "/home/a%20b/%E6%97%A5%E6%9C%AC.txt"
        );
        assert_eq!(
            percent_encode_path("/plain/path-1_2.txt"),
            "/plain/path-1_2.txt"
        );
        assert_eq!(percent_encode_path("/a#b?c"), "/a%23b%3Fc");
    }
}
