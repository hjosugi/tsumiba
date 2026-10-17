//! The global shortcut that toggles the shelf.
//!
//! Shortcuts are written the way people write them (`Ctrl+Alt+Space`) in
//! `config.toml`. Parsing is delegated to `global-hotkey`, which accepts
//! `Ctrl`/`Control`, `Alt`/`Option`, `Shift`, `Super`/`Cmd`/`Meta` and
//! `CmdOrCtrl`, case-insensitively, followed by one key.
//!
//! Wayland offers a client no way to grab a key globally, so there the service
//! reports itself unavailable and the user binds `tsumiba toggle` in their
//! desktop's keyboard settings instead.

use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use eframe::egui;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use super::cursor::is_wayland;
use crate::control::{Command, Control};

/// Parse a shortcut, requiring a modifier unless it is a function key: a bare
/// letter as a *global* shortcut would swallow that letter everywhere.
pub fn parse(spec: &str) -> Result<HotKey, String> {
    let hotkey = HotKey::from_str(spec.trim()).map_err(|e| e.to_string())?;
    let is_function_key = spec
        .trim()
        .rsplit('+')
        .next()
        .map(|k| {
            let k = k.trim().to_ascii_uppercase();
            k.len() >= 2
                && k.starts_with('F')
                && k[1..].parse::<u8>().is_ok_and(|n| (1..=24).contains(&n))
        })
        .unwrap_or(false);
    if hotkey.mods.is_empty() && !is_function_key {
        return Err(
            "a shortcut needs at least one modifier key (Ctrl, Alt, Shift or Super)".into(),
        );
    }
    Ok(hotkey)
}

/// Turn what the user just pressed into the text stored in the config. `None`
/// for combinations that are not usable as a global shortcut.
pub fn describe(modifiers: egui::Modifiers, key: egui::Key) -> Option<String> {
    let name = key_name(key)?;
    let mut parts: Vec<&str> = Vec::new();
    if modifiers.ctrl {
        parts.push("Ctrl");
    }
    if modifiers.alt {
        parts.push("Alt");
    }
    if modifiers.shift {
        parts.push("Shift");
    }
    // On macOS egui reports Command as `mac_cmd` (and mirrors it into
    // `command`); elsewhere the logo key arrives as nothing at all.
    if modifiers.mac_cmd {
        parts.push("Cmd");
    }
    parts.push(name);
    let spec = parts.join("+");
    parse(&spec).ok().map(|_| spec)
}

fn key_name(key: egui::Key) -> Option<&'static str> {
    use egui::Key::*;
    Some(match key {
        A => "A",
        B => "B",
        C => "C",
        D => "D",
        E => "E",
        F => "F",
        G => "G",
        H => "H",
        I => "I",
        J => "J",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        O => "O",
        P => "P",
        Q => "Q",
        R => "R",
        S => "S",
        T => "T",
        U => "U",
        V => "V",
        W => "W",
        X => "X",
        Y => "Y",
        Z => "Z",
        Num0 => "0",
        Num1 => "1",
        Num2 => "2",
        Num3 => "3",
        Num4 => "4",
        Num5 => "5",
        Num6 => "6",
        Num7 => "7",
        Num8 => "8",
        Num9 => "9",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        Space => "Space",
        Enter => "Enter",
        Tab => "Tab",
        Backspace => "Backspace",
        Delete => "Delete",
        Insert => "Insert",
        Home => "Home",
        End => "End",
        PageUp => "PageUp",
        PageDown => "PageDown",
        ArrowUp => "Up",
        ArrowDown => "Down",
        ArrowLeft => "Left",
        ArrowRight => "Right",
        Minus => "-",
        Equals => "=",
        Comma => ",",
        Period => ".",
        Slash => "/",
        Semicolon => ";",
        Backtick => "`",
        OpenBracket => "[",
        CloseBracket => "]",
        _ => return None,
    })
}

pub struct HotkeyService {
    manager: Option<GlobalHotKeyManager>,
    registered: Option<HotKey>,
    /// Id of the registered shortcut, read by the event handler.
    current_id: Arc<AtomicU32>,
}

impl HotkeyService {
    /// Start the service. On sessions that cannot grab keys the service still
    /// exists but is [`available`](Self::available) = false.
    pub fn new(control: Control) -> HotkeyService {
        let current_id = Arc::new(AtomicU32::new(0));
        let manager = if is_wayland() {
            log::info!("global shortcuts are not available on Wayland; use `tsumiba toggle`");
            None
        } else {
            match GlobalHotKeyManager::new() {
                Ok(manager) => Some(manager),
                Err(e) => {
                    log::warn!("global shortcuts are not available: {e}");
                    None
                }
            }
        };

        if manager.is_some() {
            let id = current_id.clone();
            GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
                if event.state == HotKeyState::Pressed && event.id == id.load(Ordering::Relaxed) {
                    control.send(Command::Toggle);
                }
            }));
        }

        HotkeyService {
            manager,
            registered: None,
            current_id,
        }
    }

    pub fn available(&self) -> bool {
        self.manager.is_some()
    }

    /// Register `spec`, replacing the previous shortcut. An empty `spec` just
    /// unregisters. On failure the previous shortcut is kept.
    pub fn apply(&mut self, spec: &str) -> Result<(), String> {
        let Some(manager) = &self.manager else {
            return if spec.trim().is_empty() {
                Ok(())
            } else {
                Err("global shortcuts are not available in this session".into())
            };
        };

        if spec.trim().is_empty() {
            if let Some(old) = self.registered.take() {
                let _ = manager.unregister(old);
            }
            self.current_id.store(0, Ordering::Relaxed);
            return Ok(());
        }

        let hotkey = parse(spec)?;
        if self.registered == Some(hotkey) {
            return Ok(());
        }
        // Registering the new one first means a failure (someone else owns that
        // combination) leaves the old shortcut working.
        manager
            .register(hotkey)
            .map_err(|e| format!("could not register {spec}: {e}"))?;
        if let Some(old) = self.registered.replace(hotkey) {
            let _ = manager.unregister(old);
        }
        self.current_id.store(hotkey.id(), Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_spellings_parse() {
        for spec in [
            "Ctrl+Alt+Space",
            "ctrl+alt+space",
            "Control+Shift+K",
            "Alt+F1",
            "Super+Shift+T",
            "Cmd+Option+Space",
            "CmdOrCtrl+Shift+Y",
            "Ctrl+Alt+Up",
            "Ctrl+1",
            "F9",
        ] {
            assert!(parse(spec).is_ok(), "{spec}");
        }
    }

    #[test]
    fn nonsense_is_rejected() {
        for spec in ["", "Ctrl+", "Ctrl+Alt", "Ctrl+Banana", "Ctrl+A+B", "+A"] {
            assert!(parse(spec).is_err(), "{spec:?}");
        }
    }

    #[test]
    fn a_bare_letter_is_not_a_global_shortcut() {
        assert!(parse("A").is_err());
        assert!(parse("Space").is_err());
        assert!(parse("Shift+A").is_ok());
        assert!(parse("F5").is_ok(), "function keys are fine alone");
        assert!(parse("F25").is_err());
    }

    #[test]
    fn captured_keys_round_trip_through_the_parser() {
        let mods = egui::Modifiers {
            ctrl: true,
            alt: true,
            ..Default::default()
        };
        let spec = describe(mods, egui::Key::Space).unwrap();
        assert_eq!(spec, "Ctrl+Alt+Space");
        assert!(parse(&spec).is_ok());
        assert_eq!(
            describe(
                egui::Modifiers {
                    shift: true,
                    ..Default::default()
                },
                egui::Key::K
            )
            .as_deref(),
            Some("Shift+K")
        );
    }

    #[test]
    fn captured_without_a_modifier_is_refused() {
        assert_eq!(describe(egui::Modifiers::NONE, egui::Key::A), None);
        assert_eq!(
            describe(egui::Modifiers::NONE, egui::Key::F5).as_deref(),
            Some("F5")
        );
    }

    #[test]
    fn unsupported_keys_are_refused() {
        let ctrl = egui::Modifiers {
            ctrl: true,
            ..Default::default()
        };
        assert_eq!(describe(ctrl, egui::Key::Escape), None);
    }
}
