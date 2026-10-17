//! A small file logger.
//!
//! On Windows the release build has no console, so anything printed to stderr is
//! lost; a start-up failure would leave nothing on screen and no trace. Log
//! lines therefore go to `tsumiba.log` in the data directory (and to stderr too
//! when there is a terminal). The file is rotated once at start-up when it
//! passes 1 MiB, so it never grows without bound.
//!
//! `TSUMIBA_LOG=debug` (or `trace`, `info`, `warn`, `error`, `off`) sets the
//! verbosity for tsumiba's own messages; other crates are held to warnings.

use std::fs::{self, File, OpenOptions};
use std::io::{IsTerminal, Write};
use std::path::Path;
use std::sync::Mutex;

use log::{LevelFilter, Log, Metadata, Record};

const ROTATE_AT: u64 = 1024 * 1024;

struct Logger {
    own: LevelFilter,
    file: Option<Mutex<File>>,
    terminal: bool,
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let limit = if metadata.target().starts_with("tsumiba") {
            self.own
        } else {
            LevelFilter::Warn
        };
        metadata.level() <= limit
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} {:<5} {}: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target(),
            record.args()
        );
        if let Some(file) = &self.file
            && let Ok(mut file) = file.lock()
        {
            let _ = file.write_all(line.as_bytes());
        }
        if self.terminal {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
    }

    fn flush(&self) {}
}

/// Install the logger. Safe to call once; later calls are ignored.
pub fn init(path: Option<&Path>) {
    let own = std::env::var("TSUMIBA_LOG")
        .ok()
        .and_then(|v| v.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Info);

    let file = path.and_then(|path| {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::metadata(path).is_ok_and(|m| m.len() > ROTATE_AT) {
            let _ = fs::rename(path, path.with_extension("log.old"));
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(Mutex::new)
    });

    let logger = Logger {
        own,
        file,
        terminal: std::io::stderr().is_terminal(),
    };
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(own.max(LevelFilter::Warn));
    }

    // A panic on a thread without a console would otherwise vanish.
    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logger(own: LevelFilter) -> Logger {
        Logger {
            own,
            file: None,
            terminal: false,
        }
    }

    fn meta(target: &str, level: log::Level) -> Metadata<'_> {
        Metadata::builder().target(target).level(level).build()
    }

    #[test]
    fn own_messages_follow_the_setting_and_others_are_held_to_warnings() {
        let l = logger(LevelFilter::Debug);
        assert!(l.enabled(&meta("tsumiba::server", log::Level::Debug)));
        assert!(!l.enabled(&meta("tsumiba::server", log::Level::Trace)));
        assert!(!l.enabled(&meta("wgpu_hal", log::Level::Info)));
        assert!(l.enabled(&meta("wgpu_hal", log::Level::Warn)));
    }

    #[test]
    fn off_silences_our_own_messages() {
        let l = logger(LevelFilter::Off);
        assert!(!l.enabled(&meta("tsumiba", log::Level::Error)));
    }
}
