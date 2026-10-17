//! Start tsumiba when the user logs in.
//!
//! Windows writes a value under `HKCU\...\Run`, macOS a launch agent, Linux a
//! `.desktop` file in `~/.config/autostart` (all through the `auto-launch`
//! crate). The entry starts tsumiba with `--background`, so it waits in the tray
//! instead of opening the panel at every login.

use auto_launch::AutoLaunchBuilder;

fn build() -> Result<auto_launch::AutoLaunch, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe
        .to_str()
        .ok_or("the executable path is not valid UTF-8")?;
    AutoLaunchBuilder::new()
        .set_app_name("tsumiba")
        .set_app_path(exe)
        .set_args(&["--background"])
        .build()
        .map_err(|e| e.to_string())
}

pub fn is_enabled() -> bool {
    build()
        .and_then(|a| a.is_enabled().map_err(|e| e.to_string()))
        .unwrap_or(false)
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let auto = build()?;
    let result = if enabled {
        auto.enable()
    } else {
        auto.disable()
    };
    result.map_err(|e| e.to_string())
}
