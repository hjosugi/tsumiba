//! Starting the app.

use std::process::ExitCode;

use tsumiba_core::api::WindowAction;
use tsumiba_core::{Config, Dirs, Store, token};

use crate::app::{self, Boot};
use crate::client::Client;
use crate::logging;
use crate::renderer::{self, Backend};

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

    let env_override = std::env::var(renderer::ENV).ok();
    let setting = renderer::effective(config.general.renderer, env_override.as_deref());
    let backend = renderer::first_choice(setting);

    let boot = Boot {
        dirs,
        config,
        config_error,
        store,
        token,
        show,
    };

    // Creating the window can panic when a system library cannot be loaded (the
    // windowing crates `dlopen` several). Turn that into an explanation.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        app::run(boot, renderer::to_eframe(backend))
    }));
    match result {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(e)) => {
            log::error!("the window could not be created: {e}");
            if renderer::should_retry_with_wgpu(
                setting,
                backend,
                renderer::is_graphics_error(&e),
                env_override.as_deref(),
            ) {
                log::warn!("OpenGL is not usable here; trying again with wgpu");
                return restart_with(Backend::Wgpu);
            }
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

/// Run this program again with another renderer and wait for it, so that to
/// whoever started us it is still one process that lives as long as the app does.
fn restart_with(backend: Backend) -> ExitCode {
    let value = match backend {
        Backend::Glow => "glow",
        Backend::Wgpu => "wgpu",
    };
    let status = std::env::current_exe().and_then(|exe| {
        std::process::Command::new(exe)
            .args(std::env::args_os().skip(1))
            .env(renderer::ENV, value)
            .status()
    });
    match status {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            log::error!("could not restart with the {value} renderer: {e}");
            eprintln!("tsumiba: could not restart with the {value} renderer: {e}");
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
