//! `config.toml`.
//!
//! Every field has a default and unknown keys are ignored, so a config file
//! written by an older or newer tsumiba keeps working. The app rewrites the file
//! when settings change in the UI; hand edits are picked up on the next start.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::api::DEFAULT_PORT;
use crate::error::{Error, IoContext, Result};
use crate::fsutil::write_atomic;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub panel: Panel,
    pub trigger: Trigger,
    pub storage: Storage,
    pub api: Api,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub language: Language,
    pub theme: Theme,
    /// Start tsumiba when you log in.
    pub launch_at_login: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Panel {
    pub show_notes: bool,
    pub show_files: bool,
    /// Width of the panel in points, clamped to the screen.
    pub width: f32,
    /// Height of the panel in points, clamped to the screen.
    pub height: f32,
    /// Slide away when the pointer leaves the panel (and you are not typing).
    pub auto_hide: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Trigger {
    /// Global shortcut, for example `Ctrl+Alt+Space`. Empty disables it.
    pub hotkey: String,
    /// Reveal the panel when the pointer rests on the top edge of the screen.
    pub edge: bool,
    /// How long the pointer must rest on the edge, in milliseconds.
    pub edge_dwell_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Storage {
    pub mode: StorageMode,
    /// Largest file the API will accept, in MiB.
    pub max_upload_mb: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Api {
    /// Run the local HTTP API used by the command line and the browser extension.
    pub enabled: bool,
    /// Loopback port. `0` lets the operating system choose one.
    pub port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Auto,
    En,
    Ja,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

/// What happens to a file you drop on the shelf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StorageMode {
    /// Keep a copy inside tsumiba's data directory. The original can be moved or
    /// deleted without affecting the shelf.
    #[default]
    Copy,
    /// Remember where the file is and do not copy it.
    Link,
}

impl Default for General {
    fn default() -> Self {
        General {
            language: Language::Auto,
            theme: Theme::System,
            launch_at_login: false,
        }
    }
}

impl Default for Panel {
    fn default() -> Self {
        Panel {
            show_notes: true,
            show_files: true,
            width: 940.0,
            height: 380.0,
            auto_hide: true,
        }
    }
}

impl Default for Trigger {
    fn default() -> Self {
        Trigger {
            hotkey: "Ctrl+Alt+Space".to_string(),
            edge: true,
            edge_dwell_ms: 250,
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Storage {
            mode: StorageMode::Copy,
            max_upload_mb: 512,
        }
    }
}

impl Default for Api {
    fn default() -> Self {
        Api {
            enabled: true,
            port: DEFAULT_PORT,
        }
    }
}

impl Config {
    /// Read the config file. A missing file is not an error: it yields the
    /// defaults. A file that cannot be parsed is an error, so a typo never
    /// silently resets your settings (the app would overwrite the file on the
    /// next change).
    pub fn load(path: &Path) -> Result<Config> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => {
                return Err(Error::Io {
                    context: format!("read {}", path.display()),
                    source: e,
                });
            }
        };
        let config: Config = toml::from_str(&text)
            .map_err(|e| Error::Config(format!("{}: {}", path.display(), e.message())))?;
        Ok(config.sanitized())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context(|| format!("create {}", parent.display()))?;
        }
        let body = toml::to_string_pretty(&self.clone().sanitized())
            .map_err(|e| Error::Config(e.to_string()))?;
        let text = format!(
            "# tsumiba configuration. See https://hjosugi.github.io/tsumiba/en/configuration.html\n\n{body}"
        );
        write_atomic(path, text.as_bytes())
    }

    /// Clamp numeric settings into ranges the UI can actually render.
    pub fn sanitized(mut self) -> Config {
        self.panel.width = clamp_f32(self.panel.width, 480.0, 3000.0, 940.0);
        self.panel.height = clamp_f32(self.panel.height, 240.0, 2000.0, 380.0);
        self.trigger.edge_dwell_ms = self.trigger.edge_dwell_ms.min(5000);
        self.storage.max_upload_mb = self.storage.max_upload_mb.clamp(1, 65_536);
        if !self.panel.show_notes && !self.panel.show_files {
            // An empty panel is never what anyone wants.
            self.panel.show_notes = true;
            self.panel.show_files = true;
        }
        self
    }
}

fn clamp_f32(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config::load(&dir.path().join("config.toml")).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn partial_file_fills_in_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[trigger]\nedge = false\n").unwrap();
        let config = Config::load(&path).unwrap();
        assert!(!config.trigger.edge);
        assert_eq!(config.trigger.hotkey, "Ctrl+Alt+Space");
        assert_eq!(config.api.port, DEFAULT_PORT);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "future = 1\n[panel]\nsparkles = true\nwidth = 700\n").unwrap();
        assert_eq!(Config::load(&path).unwrap().panel.width, 700.0);
    }

    #[test]
    fn broken_file_is_an_error_not_a_reset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[panel\nwidth = ").unwrap();
        let err = Config::load(&path).unwrap_err();
        assert!(matches!(err, Error::Config(_)), "{err}");
        assert!(err.to_string().contains("config.toml"));
    }

    #[test]
    fn wrong_types_are_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[storage]\nmode = \"teleport\"\n").unwrap();
        assert!(Config::load(&path).is_err());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/config.toml");
        let mut config = Config::default();
        config.general.language = Language::Ja;
        config.general.theme = Theme::Dark;
        config.storage.mode = StorageMode::Link;
        config.trigger.hotkey = "Ctrl+Shift+K".into();
        config.api.port = 5555;
        config.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), config);
        assert!(fs::read_to_string(&path).unwrap().starts_with("# tsumiba"));
    }

    #[test]
    fn numbers_are_clamped() {
        let mut config = Config::default();
        config.panel.width = 10.0;
        config.panel.height = f32::NAN;
        config.trigger.edge_dwell_ms = u32::MAX;
        config.storage.max_upload_mb = 0;
        let config = config.sanitized();
        assert_eq!(config.panel.width, 480.0);
        assert_eq!(config.panel.height, 380.0);
        assert_eq!(config.trigger.edge_dwell_ms, 5000);
        assert_eq!(config.storage.max_upload_mb, 1);
    }

    #[test]
    fn at_least_one_panel_stays_visible() {
        let mut config = Config::default();
        config.panel.show_notes = false;
        config.panel.show_files = false;
        let config = config.sanitized();
        assert!(config.panel.show_notes && config.panel.show_files);
    }
}
