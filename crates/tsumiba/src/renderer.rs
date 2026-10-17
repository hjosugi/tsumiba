//! Choosing how the window is drawn.
//!
//! OpenGL is the default everywhere: small, quick to start, and supported by every
//! desktop with a graphics driver. The exception is a machine whose OpenGL is
//! Windows' built-in 1.1 software stub — a virtual machine or remote desktop
//! session without a graphics driver — where the window cannot be created at all.
//! Windows builds therefore also contain the wgpu renderer (DirectX 12, with its
//! software rasteriser as the last resort) and use it when OpenGL fails.
//!
//! The window toolkit cannot start a second event loop in the same process, so the
//! retry is a fresh process started with `TSUMIBA_RENDERER=wgpu`.

use tsumiba_core::config::Renderer as Setting;

/// Overrides `general.renderer` from `config.toml`. Also how the retry knows it is one.
pub const ENV: &str = "TSUMIBA_RENDERER";

/// Whether this build contains the wgpu renderer.
pub const HAS_WGPU: bool = cfg!(windows);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Glow,
    Wgpu,
}

/// The configured setting, unless the environment overrides it.
pub fn effective(config: Setting, env: Option<&str>) -> Setting {
    match env.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
        Some("auto") => Setting::Auto,
        Some("glow" | "opengl" | "gl") => Setting::Glow,
        Some("wgpu") => Setting::Wgpu,
        _ => config,
    }
}

/// What to try first.
pub fn first_choice(setting: Setting) -> Backend {
    match setting {
        Setting::Wgpu if HAS_WGPU => Backend::Wgpu,
        _ => Backend::Glow,
    }
}

/// Whether a start that failed with `error_is_graphics` should be retried with wgpu.
///
/// Only when nothing was asked for (`auto`), OpenGL is what failed, this build has
/// wgpu, and this is not already the retry: a user who forced `glow` gets the
/// error, and a retry that fails does not loop.
pub fn should_retry_with_wgpu(
    setting: Setting,
    tried: Backend,
    error_is_graphics: bool,
    env_override: Option<&str>,
) -> bool {
    HAS_WGPU
        && error_is_graphics
        && setting == Setting::Auto
        && tried == Backend::Glow
        && env_override.is_none()
}

/// Whether `error` is the kind that a different renderer can get around.
pub fn is_graphics_error(error: &eframe::Error) -> bool {
    matches!(
        error,
        eframe::Error::OpenGL(_) | eframe::Error::NoGlutinConfigs(..) | eframe::Error::Glutin(_)
    )
}

pub fn to_eframe(backend: Backend) -> eframe::Renderer {
    match backend {
        #[cfg(windows)]
        Backend::Wgpu => eframe::Renderer::Wgpu,
        // Without the feature there is only OpenGL; `first_choice` never asks for
        // anything else, but the match has to be total.
        _ => eframe::Renderer::Glow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_environment_overrides_the_config() {
        assert_eq!(effective(Setting::Auto, None), Setting::Auto);
        assert_eq!(effective(Setting::Glow, None), Setting::Glow);
        assert_eq!(effective(Setting::Auto, Some("wgpu")), Setting::Wgpu);
        assert_eq!(effective(Setting::Wgpu, Some("GLOW")), Setting::Glow);
        assert_eq!(effective(Setting::Wgpu, Some(" opengl ")), Setting::Glow);
        assert_eq!(effective(Setting::Wgpu, Some("auto")), Setting::Auto);
    }

    #[test]
    fn nonsense_in_the_environment_is_ignored() {
        assert_eq!(effective(Setting::Glow, Some("vulkan-ish")), Setting::Glow);
        assert_eq!(effective(Setting::Wgpu, Some("")), Setting::Wgpu);
    }

    #[test]
    fn auto_starts_with_opengl() {
        assert_eq!(first_choice(Setting::Auto), Backend::Glow);
        assert_eq!(first_choice(Setting::Glow), Backend::Glow);
    }

    #[test]
    fn wgpu_is_used_first_only_where_it_exists() {
        let expected = if HAS_WGPU {
            Backend::Wgpu
        } else {
            Backend::Glow
        };
        assert_eq!(first_choice(Setting::Wgpu), expected);
    }

    #[test]
    fn a_graphics_failure_on_auto_retries_where_wgpu_exists() {
        assert_eq!(
            should_retry_with_wgpu(Setting::Auto, Backend::Glow, true, None),
            HAS_WGPU
        );
    }

    #[test]
    fn the_retry_does_not_happen_when_it_should_not() {
        // Something other than a graphics error.
        assert!(!should_retry_with_wgpu(
            Setting::Auto,
            Backend::Glow,
            false,
            None
        ));
        // The user forced OpenGL.
        assert!(!should_retry_with_wgpu(
            Setting::Glow,
            Backend::Glow,
            true,
            None
        ));
        // Already the retry (the environment names the renderer): no loop.
        assert!(!should_retry_with_wgpu(
            Setting::Wgpu,
            Backend::Wgpu,
            true,
            Some("wgpu")
        ));
        assert!(!should_retry_with_wgpu(
            Setting::Wgpu,
            Backend::Glow,
            true,
            Some("wgpu")
        ));
        // wgpu itself failed.
        assert!(!should_retry_with_wgpu(
            Setting::Auto,
            Backend::Wgpu,
            true,
            None
        ));
    }
}
