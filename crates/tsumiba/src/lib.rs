//! tsumiba: a memo and file shelf that slides in from the screen edge.
//!
//! The binary is a thin wrapper around this library so that the pieces can be
//! unit-tested and so integration tests can start a real server.

pub mod app;
pub mod cli;
pub mod client;
pub mod control;
pub mod i18n;
pub mod launch;
pub mod logging;
pub mod platform;
pub mod server;
pub mod winconsole;
