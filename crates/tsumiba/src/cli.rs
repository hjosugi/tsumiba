//! The command line.
//!
//! Every command that changes the shelf talks to the running app when there is
//! one (so its window stays current and one process owns the files) and falls
//! back to working on the store directly when there is not, so `tsumiba note`
//! and `tsumiba add` work in scripts and over SSH with nothing running.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use tsumiba_core::api::{FileDto, NoteDto, WindowAction};
use tsumiba_core::config::StorageMode;
use tsumiba_core::runtime::ServerInfo;
use tsumiba_core::{Config, Dirs, Store, token};

use crate::client::{Client, ClientError};
use crate::control::{Command, Control};
use crate::server::{ApiContext, Server, file_dto, note_dto};

#[derive(Debug, Parser)]
#[command(
    name = "tsumiba",
    version,
    about = "A memo and file shelf that slides in from the screen edge",
    long_about = "tsumiba keeps notes and files one gesture away.\n\n\
        With no command it opens the shelf (starting it if needed). Bind `tsumiba toggle` \
        to a key in your desktop's settings if global shortcuts are not available there \
        (Wayland).\n\nDocumentation: https://hjosugi.github.io/tsumiba/"
)]
pub struct Cli {
    /// Start in the background (tray only) instead of opening the shelf.
    #[arg(long)]
    pub background: bool,

    #[command(subcommand)]
    pub command: Option<Cmd>,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Open the shelf, starting tsumiba if it is not running (the default).
    Run,
    /// Show the shelf (starts tsumiba if needed).
    Show,
    /// Hide the shelf.
    Hide,
    /// Show the shelf if it is hidden, hide it if it is shown (starts tsumiba if needed).
    Toggle,
    /// Add a note. The text comes from the arguments, or from standard input.
    Note {
        /// The note's text. Use `-` or give nothing to read standard input.
        text: Vec<String>,
        /// Pin the note to the top of the list.
        #[arg(long)]
        pin: bool,
    },
    /// Add files (or folders) to the shelf.
    Add {
        paths: Vec<PathBuf>,
        /// Remember where the files are instead of copying them.
        #[arg(long)]
        link: bool,
    },
    /// List notes and files.
    List {
        /// Only notes.
        #[arg(long)]
        notes: bool,
        /// Only files.
        #[arg(long)]
        files: bool,
        /// Only notes containing this text.
        #[arg(short, long)]
        query: Option<String>,
        /// Print JSON instead of tab-separated text.
        #[arg(long)]
        json: bool,
    },
    /// Quit the running tsumiba.
    Quit,
    /// Print the token the browser extension needs to pair with this app.
    Token {
        /// Create a new token (signs out every paired browser).
        #[arg(long)]
        regenerate: bool,
    },
    /// Print where tsumiba keeps its files.
    Paths,
    /// Run the local API without a window (for servers and tests).
    Serve {
        /// Listen on this port instead of the configured one (0 = any free port).
        #[arg(long)]
        port: Option<u16>,
    },
}

/// What `main` should do once the command has run.
pub enum Next {
    Done,
    /// Start the app. `show` is whether to open the panel.
    Gui {
        show: bool,
    },
}

struct Env {
    dirs: Dirs,
    config: Config,
}

impl Env {
    fn load() -> Result<Env> {
        let dirs = Dirs::discover()?;
        dirs.ensure()?;
        let config = Config::load(&dirs.config_file()).unwrap_or_else(|e| {
            eprintln!("tsumiba: warning: {e}; using the defaults");
            Config::default()
        });
        Ok(Env { dirs, config })
    }

    fn client(&self) -> Option<Client> {
        Client::discover(&self.dirs, &self.config)
    }

    fn store(&self) -> Result<Store> {
        Store::open(&self.dirs.data).context("could not open the shelf")
    }
}

pub fn run(command: Cmd) -> Result<Next> {
    let env = Env::load()?;
    match command {
        Cmd::Run => Ok(Next::Gui { show: true }),
        Cmd::Show => window(&env, WindowAction::Show),
        Cmd::Toggle => window(&env, WindowAction::Toggle),
        Cmd::Hide => match env.client() {
            Some(client) => {
                client.window(WindowAction::Hide)?;
                Ok(Next::Done)
            }
            None => bail!("tsumiba is not running"),
        },
        Cmd::Quit => match env.client() {
            Some(client) => {
                client.quit()?;
                Ok(Next::Done)
            }
            None => bail!("tsumiba is not running"),
        },
        Cmd::Note { text, pin } => note(&env, text, pin).map(|()| Next::Done),
        Cmd::Add { paths, link } => add(&env, paths, link).map(|()| Next::Done),
        Cmd::List {
            notes,
            files,
            query,
            json,
        } => list(&env, notes, files, query.as_deref().unwrap_or(""), json).map(|()| Next::Done),
        Cmd::Token { regenerate } => {
            let token = if regenerate {
                let token = token::regenerate(&env.dirs.token_file())?;
                eprintln!(
                    "tsumiba: the token was replaced. Restart tsumiba and pair your browsers again."
                );
                token
            } else {
                token::load_or_create(&env.dirs.token_file())?
            };
            println!("{token}");
            Ok(Next::Done)
        }
        Cmd::Paths => {
            println!("data    {}", env.dirs.data.display());
            println!("config  {}", env.dirs.config_file().display());
            println!("token   {}", env.dirs.token_file().display());
            println!("log     {}", env.dirs.log_file().display());
            Ok(Next::Done)
        }
        Cmd::Serve { port } => serve(env, port).map(|()| Next::Done),
    }
}

fn window(env: &Env, action: WindowAction) -> Result<Next> {
    match env.client() {
        Some(client) => {
            client.window(action)?;
            Ok(Next::Done)
        }
        // Nothing is running: starting the app *is* showing it.
        None => Ok(Next::Gui { show: true }),
    }
}

fn note(env: &Env, text: Vec<String>, pin: bool) -> Result<()> {
    let body = if text.is_empty() || text == ["-"] {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .context("could not read standard input")?;
        input
    } else {
        text.join(" ")
    };
    if body.trim().is_empty() {
        bail!("nothing to save: give the text as arguments or pipe it in");
    }

    let id = match env.client() {
        Some(client) => client.create_note(&body, pin)?.id,
        None => {
            let store = env.store()?;
            let note = store.add_note(&body)?;
            if pin {
                store.set_note_pinned(&note.id, true)?;
            }
            note.id
        }
    };
    println!("{id}");
    Ok(())
}

fn add(env: &Env, paths: Vec<PathBuf>, link: bool) -> Result<()> {
    if paths.is_empty() {
        bail!("give at least one file or folder");
    }
    let absolute: Vec<PathBuf> = paths
        .iter()
        .map(|p| std::path::absolute(p).with_context(|| format!("cannot resolve {}", p.display())))
        .collect::<Result<_>>()?;

    let mut failures = 0;
    match env.client() {
        Some(client) => {
            let strings = absolute
                .iter()
                .map(|p| {
                    p.to_str()
                        .map(str::to_string)
                        .with_context(|| format!("{} is not valid UTF-8", p.display()))
                })
                .collect::<Result<Vec<_>>>()?;
            let result = client.import(strings, link)?;
            for file in &result.added {
                println!("{}\t{}", file.id, file.name);
            }
            for failure in &result.failed {
                eprintln!("tsumiba: {}: {}", failure.path, failure.error);
                failures += 1;
            }
        }
        None => {
            let store = env.store()?;
            let mode = if link {
                StorageMode::Link
            } else {
                env.config.storage.mode
            };
            for path in &absolute {
                match store.add_path(path, mode) {
                    Ok(file) => println!("{}\t{}", file.id, file.name),
                    Err(e) => {
                        eprintln!("tsumiba: {}: {e}", path.display());
                        failures += 1;
                    }
                }
            }
        }
    }
    if failures > 0 {
        bail!("{failures} of {} could not be added", absolute.len());
    }
    Ok(())
}

fn list(env: &Env, only_notes: bool, only_files: bool, query: &str, json: bool) -> Result<()> {
    let want_notes = only_notes || !only_files;
    let want_files = only_files || !only_notes;

    let (notes, files): (Vec<NoteDto>, Vec<FileDto>) = match env.client() {
        Some(client) => (
            if want_notes {
                client.notes(query)?
            } else {
                Vec::new()
            },
            if want_files {
                client.files()?
            } else {
                Vec::new()
            },
        ),
        None => {
            let store = env.store()?;
            (
                if want_notes {
                    store
                        .notes()
                        .iter()
                        .filter(|n| n.matches(query))
                        .map(note_dto)
                        .collect()
                } else {
                    Vec::new()
                },
                if want_files {
                    store.files().iter().map(file_dto).collect()
                } else {
                    Vec::new()
                },
            )
        }
    };

    let mut out = std::io::stdout().lock();
    if json {
        let value = serde_json::json!({ "notes": notes, "files": files });
        writeln!(out, "{}", serde_json::to_string_pretty(&value)?)?;
        return Ok(());
    }
    for note in &notes {
        let title = if note.title.is_empty() {
            "(untitled)"
        } else {
            &note.title
        };
        writeln!(
            out,
            "note\t{}\t{}{}",
            note.id,
            if note.pinned { "📌 " } else { "" },
            title
        )?;
    }
    for file in &files {
        writeln!(out, "file\t{}\t{}\t{}", file.id, file.name, file.size)?;
    }
    Ok(())
}

/// Run the API without a window until asked to quit.
fn serve(env: Env, port: Option<u16>) -> Result<()> {
    if env.client().is_some() {
        bail!("tsumiba is already running");
    }
    let store = env.store()?;
    let token = token::load_or_create(&env.dirs.token_file())?;
    let (control, rx) = Control::new();
    let api = ApiContext {
        store,
        control,
        token,
        config: Arc::new(RwLock::new(env.config.clone())),
    };
    let server = Server::start(api, port.unwrap_or(env.config.api.port))
        .map_err(|e| anyhow::anyhow!("could not start the API: {e}"))?;

    ServerInfo {
        port: server.port(),
        pid: std::process::id(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
    .write(&env.dirs.runtime_file())?;

    println!("listening on 127.0.0.1:{}", server.port());
    std::io::stdout().flush().ok();

    // Window commands mean nothing without a window; only Quit matters.
    while let Ok(command) = rx.recv() {
        if command == Command::Quit {
            break;
        }
    }
    server.shutdown();
    ServerInfo::remove(&env.dirs.runtime_file())?;
    Ok(())
}

/// Turn a client error into the message a user should see.
pub fn describe(error: &anyhow::Error) -> String {
    match error.downcast_ref::<ClientError>() {
        Some(ClientError::NotRunning) => "tsumiba is not running".to_string(),
        _ => format!("{error:#}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_command_line_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_the_documented_forms() {
        let cli = Cli::try_parse_from(["tsumiba", "--background"]).unwrap();
        assert!(cli.background && cli.command.is_none());

        let cli = Cli::try_parse_from(["tsumiba", "note", "buy", "milk", "--pin"]).unwrap();
        match cli.command {
            Some(Cmd::Note { text, pin }) => {
                assert_eq!(text, ["buy", "milk"]);
                assert!(pin);
            }
            other => panic!("{other:?}"),
        }

        let cli = Cli::try_parse_from(["tsumiba", "add", "--link", "a.txt", "dir"]).unwrap();
        match cli.command {
            Some(Cmd::Add { paths, link }) => {
                assert_eq!(paths.len(), 2);
                assert!(link);
            }
            other => panic!("{other:?}"),
        }

        let cli = Cli::try_parse_from(["tsumiba", "list", "--files", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Cmd::List {
                files: true,
                json: true,
                notes: false,
                ..
            })
        ));
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert!(Cli::try_parse_from(["tsumiba", "frobnicate"]).is_err());
    }
}
