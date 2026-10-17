//! Starting the app.

use std::process::ExitCode;

use tsumiba_core::api::WindowAction;
use tsumiba_core::{Config, Dirs, Store, token};

use crate::app::{self, Boot};
use crate::client::Client;
use crate::logging;

/// Open the shelf. `show` is whether the panel should appear at once (a manual
/// launch) or the app should wait in the tray (start at login).
///
/// If tsumiba is already running, this does not start a second copy: it asks the
/// running one to show itself and returns.
pub fn launch(show: bool) -> ExitCode {
    let dirs = match Dirs::discover().and_then(|d| d.ensure().map(|()| d)) {
        Ok(dirs) => dirs,
        Err(e) => {
            eprintln!("tsumiba: {e}");
            return ExitCode::FAILURE;
        }
    };
    logging::init(Some(&dirs.log_file()));

    let (config, config_error) = match Config::load(&dirs.config_file()) {
        Ok(config) => (config, None),
        Err(e) => {
            log::warn!("{e}");
            (Config::default(), Some(e.to_string()))
        }
    };

    if let Some(running) = Client::discover(&dirs, &config) {
        if show {
            let _ = running.window(WindowAction::Show);
        }
        return ExitCode::SUCCESS;
    }

    let store = match Store::open(&dirs.data) {
        Ok(store) => store,
        Err(e) => {
            log::error!("could not open the shelf: {e}");
            eprintln!("tsumiba: could not open the shelf: {e}");
            return ExitCode::FAILURE;
        }
    };
    let token = match token::load_or_create(&dirs.token_file()) {
        Ok(token) => token,
        Err(e) => {
            log::error!("could not read or create the API token: {e}");
            eprintln!("tsumiba: {e}");
            return ExitCode::FAILURE;
        }
    };

    log::info!("starting tsumiba {}", env!("CARGO_PKG_VERSION"));
    let result = app::run(Boot {
        dirs,
        config,
        config_error,
        store,
        token,
        show,
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("the window could not be created: {e}");
            eprintln!("tsumiba: the window could not be created: {e}");
            ExitCode::FAILURE
        }
    }
}
