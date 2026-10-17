// A release build on Windows is a GUI program: no console window appears when it
// starts at login. `winconsole` reattaches to a terminal when run with arguments.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::process::ExitCode;

use clap::Parser;
use tsumiba::cli::{self, Cli, Cmd, Next};
use tsumiba::{launch, winconsole};

fn main() -> ExitCode {
    if std::env::args_os().len() > 1 {
        winconsole::attach_parent();
    }
    let cli = Cli::parse();

    match cli.command {
        None => launch::launch(!cli.background),
        Some(Cmd::Run) => launch::launch(!cli.background),
        Some(command) => match cli::run(command) {
            Ok(Next::Done) => ExitCode::SUCCESS,
            Ok(Next::Gui { show }) => launch::launch(show),
            Err(error) => {
                eprintln!("tsumiba: {}", cli::describe(&error));
                ExitCode::FAILURE
            }
        },
    }
}
