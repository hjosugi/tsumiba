//! Where tsumiba keeps its data and its configuration.
//!
//! By default the operating system's conventional per-user locations are used
//! (through the `directories` crate):
//!
//! | OS      | data                                                   | config                                                 |
//! | ------- | ------------------------------------------------------ | ------------------------------------------------------ |
//! | Linux   | `$XDG_DATA_HOME/tsumiba`                               | `$XDG_CONFIG_HOME/tsumiba`                             |
//! | macOS   | `~/Library/Application Support/io.github.hjosugi.tsumiba` | same                                                |
//! | Windows | `%APPDATA%\hjosugi\tsumiba\data`                       | `%APPDATA%\hjosugi\tsumiba\config`                     |
//!
//! Three environment variables override that: `TSUMIBA_DATA_DIR`,
//! `TSUMIBA_CONFIG_DIR`, and `TSUMIBA_HOME` (which sets both, to `data` and
//! `config` below it). The specific ones win over `TSUMIBA_HOME`.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::error::{Error, Result};

pub const ENV_DATA_DIR: &str = "TSUMIBA_DATA_DIR";
pub const ENV_CONFIG_DIR: &str = "TSUMIBA_CONFIG_DIR";
pub const ENV_HOME: &str = "TSUMIBA_HOME";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// Notes, files, the index and the log.
    pub data: PathBuf,
    /// `config.toml`, the API token and the running-server record.
    pub config: PathBuf,
}

impl Dirs {
    /// Resolve the directories from the process environment.
    pub fn discover() -> Result<Dirs> {
        Self::discover_with(|key| std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from))
    }

    /// [`discover`](Self::discover) with the environment injected, so tests do
    /// not have to mutate process-global state.
    pub fn discover_with(env: impl Fn(&str) -> Option<PathBuf>) -> Result<Dirs> {
        let home = env(ENV_HOME);
        let data = env(ENV_DATA_DIR).or_else(|| home.as_ref().map(|h| h.join("data")));
        let config = env(ENV_CONFIG_DIR).or_else(|| home.as_ref().map(|h| h.join("config")));

        if let (Some(data), Some(config)) = (&data, &config) {
            return Ok(Dirs::from_roots(data, config));
        }

        let project = ProjectDirs::from("io.github", "hjosugi", "tsumiba").ok_or_else(|| {
            Error::Invalid(format!(
                "cannot determine a home directory; set {ENV_HOME} to choose where tsumiba keeps its files"
            ))
        })?;
        Ok(Dirs {
            data: data.unwrap_or_else(|| project.data_dir().to_path_buf()),
            config: config.unwrap_or_else(|| project.config_dir().to_path_buf()),
        })
    }

    pub fn from_roots(data: impl Into<PathBuf>, config: impl Into<PathBuf>) -> Dirs {
        Dirs {
            data: data.into(),
            config: config.into(),
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config.join("config.toml")
    }

    pub fn token_file(&self) -> PathBuf {
        self.config.join("token")
    }

    /// Written by a running instance so the command line can find it.
    pub fn runtime_file(&self) -> PathBuf {
        self.config.join("server.json")
    }

    pub fn log_file(&self) -> PathBuf {
        self.data.join("tsumiba.log")
    }

    pub fn ensure(&self) -> Result<()> {
        for dir in [&self.data, &self.config] {
            std::fs::create_dir_all(dir).map_err(|source| Error::Io {
                context: format!("create {}", dir.display()),
                source,
            })?;
        }
        Ok(())
    }
}

impl AsRef<Path> for Dirs {
    fn as_ref(&self) -> &Path {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<PathBuf> + 'a {
        move |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| PathBuf::from(v))
        }
    }

    #[test]
    fn home_sets_both_directories() {
        let dirs = Dirs::discover_with(env_of(&[(ENV_HOME, "/x")])).unwrap();
        assert_eq!(dirs.data, PathBuf::from("/x/data"));
        assert_eq!(dirs.config, PathBuf::from("/x/config"));
    }

    #[test]
    fn specific_variables_win_over_home() {
        let dirs = Dirs::discover_with(env_of(&[
            (ENV_HOME, "/x"),
            (ENV_DATA_DIR, "/d"),
            (ENV_CONFIG_DIR, "/c"),
        ]))
        .unwrap();
        assert_eq!(dirs, Dirs::from_roots("/d", "/c"));
    }

    #[test]
    fn one_specific_variable_overrides_only_its_half() {
        let dirs = Dirs::discover_with(env_of(&[(ENV_HOME, "/x"), (ENV_DATA_DIR, "/d")])).unwrap();
        assert_eq!(dirs.data, PathBuf::from("/d"));
        assert_eq!(dirs.config, PathBuf::from("/x/config"));
    }

    #[test]
    fn file_names_live_in_the_right_directory() {
        let dirs = Dirs::from_roots("/d", "/c");
        assert_eq!(dirs.config_file(), PathBuf::from("/c/config.toml"));
        assert_eq!(dirs.token_file(), PathBuf::from("/c/token"));
        assert_eq!(dirs.runtime_file(), PathBuf::from("/c/server.json"));
        assert_eq!(dirs.log_file(), PathBuf::from("/d/tsumiba.log"));
    }

    #[test]
    fn ensure_creates_both_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let dirs = Dirs::from_roots(tmp.path().join("a/b"), tmp.path().join("c/d"));
        dirs.ensure().unwrap();
        assert!(dirs.data.is_dir() && dirs.config.is_dir());
    }
}
