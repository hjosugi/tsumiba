//! The pointer's position on the whole desktop, and whether the primary button
//! is down.
//!
//! No platform needs a special permission for this: `GetCursorPos` on Windows,
//! `CGEventGetLocation` on macOS, `XQueryPointer` on X11. Wayland deliberately
//! gives a client no way to see the pointer outside its own surfaces, so there
//! [`PointerSource::new`] returns `None` and the top-edge trigger is simply
//! unavailable (the shortcut and `tsumiba toggle` still work).

/// A reading of the pointer, in native desktop coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer {
    pub x: i32,
    pub y: i32,
    pub primary_down: bool,
}

pub struct PointerSource(imp::Source);

impl PointerSource {
    /// `None` when this session offers no way to read the pointer.
    pub fn new() -> Option<PointerSource> {
        imp::Source::new().map(PointerSource)
    }

    pub fn read(&mut self) -> Option<Pointer> {
        self.0.read()
    }
}

/// Whether the pointer can be read at all in this session. Cheap; used by the
/// settings screen to explain why the edge trigger is greyed out.
pub fn available() -> bool {
    imp::available()
}

/// Whether the session is Wayland, where windows cannot position or raise
/// themselves.
pub fn is_wayland() -> bool {
    #[cfg(target_os = "linux")]
    {
        wayland_from_env(|k| std::env::var(k).ok())
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// winit's own rule: honour an explicit backend request, otherwise a session
/// with a Wayland display is a Wayland session.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn wayland_from_env(env: impl Fn(&str) -> Option<String>) -> bool {
    match env("WINIT_UNIX_BACKEND").as_deref() {
        Some("x11") => return false,
        Some("wayland") => return true,
        _ => {}
    }
    env("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}

#[cfg(target_os = "linux")]
mod imp {
    use super::{Pointer, is_wayland};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, KeyButMask};
    use x11rb::rust_connection::RustConnection;

    pub struct Source {
        conn: RustConnection,
        root: u32,
    }

    pub fn available() -> bool {
        !is_wayland() && std::env::var_os("DISPLAY").is_some()
    }

    impl Source {
        pub fn new() -> Option<Source> {
            if !available() {
                return None;
            }
            let (conn, screen) = RustConnection::connect(None).ok()?;
            let root = conn.setup().roots.get(screen)?.root;
            Some(Source { conn, root })
        }

        pub fn read(&mut self) -> Option<Pointer> {
            let reply = self.conn.query_pointer(self.root).ok()?.reply().ok()?;
            Some(Pointer {
                x: i32::from(reply.root_x),
                y: i32::from(reply.root_y),
                primary_down: reply.mask.contains(KeyButMask::BUTTON1),
            })
        }
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::Pointer;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    pub struct Source;

    pub fn available() -> bool {
        true
    }

    impl Source {
        pub fn new() -> Option<Source> {
            Some(Source)
        }

        pub fn read(&mut self) -> Option<Pointer> {
            let mut point = POINT { x: 0, y: 0 };
            // SAFETY: `point` is a valid, writable POINT for the duration of the call.
            if unsafe { GetCursorPos(&mut point) } == 0 {
                return None;
            }
            // The high bit of the result is "currently down".
            // SAFETY: GetAsyncKeyState takes a plain integer and has no preconditions.
            let down = unsafe { GetAsyncKeyState(i32::from(VK_LBUTTON)) } < 0;
            Some(Pointer {
                x: point.x,
                y: point.y,
                primary_down: down,
            })
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::Pointer;
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceButtonState(state_id: i32, button: u32) -> u8;
    }

    // kCGEventSourceStateCombinedSessionState and kCGMouseButtonLeft.
    const COMBINED_SESSION_STATE: i32 = 0;
    const LEFT_BUTTON: u32 = 0;

    pub struct Source;

    pub fn available() -> bool {
        true
    }

    impl Source {
        pub fn new() -> Option<Source> {
            Some(Source)
        }

        pub fn read(&mut self) -> Option<Pointer> {
            let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
            let event = CGEvent::new(source).ok()?;
            let location = event.location();
            // SAFETY: both arguments are plain integers naming constants.
            let down =
                unsafe { CGEventSourceButtonState(COMBINED_SESSION_STATE, LEFT_BUTTON) } != 0;
            Some(Pointer {
                x: location.x as i32,
                y: location.y as i32,
                primary_down: down,
            })
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod imp {
    use super::Pointer;

    pub struct Source;

    pub fn available() -> bool {
        false
    }

    impl Source {
        pub fn new() -> Option<Source> {
            None
        }

        pub fn read(&mut self) -> Option<Pointer> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pairs
                .iter()
                .find(|(key, _)| *key == k)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn wayland_detection_follows_winit() {
        assert!(wayland_from_env(env_of(&[(
            "WAYLAND_DISPLAY",
            "wayland-0"
        )])));
        assert!(!wayland_from_env(env_of(&[])));
        assert!(!wayland_from_env(env_of(&[("WAYLAND_DISPLAY", "")])));
        // An explicit backend choice wins in both directions.
        assert!(!wayland_from_env(env_of(&[
            ("WAYLAND_DISPLAY", "wayland-0"),
            ("WINIT_UNIX_BACKEND", "x11")
        ])));
        assert!(wayland_from_env(env_of(&[(
            "WINIT_UNIX_BACKEND",
            "wayland"
        )])));
    }
}
