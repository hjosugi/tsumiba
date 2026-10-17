//! Storage, configuration and wire types behind [tsumiba](https://github.com/hjosugi/tsumiba),
//! a memo and file shelf that slides in from the screen edge.
//!
//! This crate has no user interface and no network code. It is what the app,
//! the command line and the tests share:
//!
//! - [`store`] — notes and files on disk ([`Store`])
//! - [`config`] — `config.toml`
//! - [`paths`] — where everything lives
//! - [`api`] — request and response types of the local HTTP API
//! - [`token`] and [`runtime`] — the pairing secret and the running-instance record

pub mod api;
pub mod config;
pub mod error;
pub mod fsutil;
pub mod id;
pub mod paths;
pub mod runtime;
pub mod store;
pub mod token;

pub use config::Config;
pub use error::{Error, Result};
pub use paths::Dirs;
pub use store::{Deleted, DeletedKind, Note, Staged, Store, StoredFile};
