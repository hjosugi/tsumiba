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
    // Creating the window can panic when a system library cannot be loaded (the
    // windowing crates `dlopen` several). Turn that into an explanation.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        app::run(Boot {
            dirs,
            config,
            config_error,
            store,
            token,
            show,
        })
    }));
    match result {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(e)) => {
            log::error!("the window could not be created: {e}");
            eprintln!("tsumiba: the window could not be created: {e}");
            ExitCode::FAILURE
        }
        Err(panic) => {
            let message = panic_message(&panic);
            let hint = missing_library_hint(&message);
            log::error!("the window could not be created: {message}. {hint}");
            eprintln!("tsumiba: the window could not be created: {message}\n{hint}");
            ExitCode::FAILURE
        }
    }
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("unknown error")
        .to_string()
}

/// What to do about a library that could not be loaded.
pub fn missing_library_hint(message: &str) -> String {
    if message.contains("libxkbcommon-x11") {
        "tsumiba needs the X11 keyboard library. Install it with your package manager: \
         libxkbcommon-x11-0 (Debian, Ubuntu), libxkbcommon-x11 (Fedora, Arch)."
            .to_string()
    } else if message.contains("libxkbcommon") {
        "tsumiba needs the keyboard library libxkbcommon. Install it with your package manager."
            .to_string()
    } else if message.contains("could not be loaded") {
        "A system library that tsumiba loads at run time is missing; the name is in the message above. \
         See the Linux requirements in the documentation."
            .to_string()
    } else {
        "Run with TSUMIBA_LOG=debug and see the log file (`tsumiba paths`) for details.".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_keyboard_library_is_explained() {
        let hint = missing_library_hint("Library libxkbcommon-x11.so could not be loaded.");
        assert!(hint.contains("libxkbcommon-x11-0"), "{hint}");
        assert!(
            missing_library_hint("Library libxkbcommon.so could not be loaded")
                .contains("libxkbcommon")
        );
    }

    #[test]
    fn an_unknown_library_still_gets_advice() {
        assert!(
            missing_library_hint("Library libfoo.so could not be loaded")
                .contains("system library")
        );
        assert!(missing_library_hint("something else").contains("TSUMIBA_LOG"));
    }
}
