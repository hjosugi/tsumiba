//! The tray (notification-area, menu-bar) icon.
//!
//! The icon is how you reach tsumiba when nothing is on screen: left-click
//! toggles the shelf, the menu has the rest. `tray-icon` provides one API for
//! all three systems: native menus on Windows and macOS, a StatusNotifierItem
//! over D-Bus on Linux (so GNOME needs its AppIndicator extension, like every
//! other tray application there).

use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::control::{Command, Control};
use crate::i18n::Strings;

const APP_ICON: &[u8] = include_bytes!("../../../../assets/icons/tsumiba-64.png");
const MENU_BAR_GLYPH: &[u8] = include_bytes!("../../../../assets/icons/tray-template.png");

/// macOS draws a monochrome template image in the menu bar; everywhere else the
/// full-colour icon is used.
fn tray_image() -> &'static [u8] {
    if cfg!(target_os = "macos") {
        MENU_BAR_GLYPH
    } else {
        APP_ICON
    }
}

pub struct Tray {
    // Dropping the TrayIcon removes it from the tray, so it has to live as long
    // as the app does.
    _icon: TrayIcon,
    toggle: MenuItem,
    settings: MenuItem,
    quit: MenuItem,
}

impl Tray {
    pub fn new(control: Control, strings: &Strings) -> Result<Tray, String> {
        let toggle = MenuItem::new(strings.tray_toggle, true, None);
        let settings = MenuItem::new(strings.tray_settings, true, None);
        let quit = MenuItem::new(strings.tray_quit, true, None);

        let menu = Menu::new();
        menu.append_items(&[&toggle, &settings, &PredefinedMenuItem::separator(), &quit])
            .map_err(|e| e.to_string())?;

        let toggle_id: MenuId = toggle.id().clone();
        let settings_id: MenuId = settings.id().clone();
        let quit_id: MenuId = quit.id().clone();
        {
            let control = control.clone();
            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                if event.id == toggle_id {
                    control.send(Command::Toggle);
                } else if event.id == settings_id {
                    control.send(Command::ShowSettings);
                } else if event.id == quit_id {
                    control.send(Command::Quit);
                }
            }));
        }
        {
            let control = control.clone();
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    control.send(Command::Toggle);
                }
            }));
        }

        let mut builder = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(strings.tray_tooltip);

        let image = load_icon(tray_image())?;
        #[cfg(target_os = "macos")]
        {
            // The menu bar draws a template image in the right colour for the
            // current appearance; the menu opens on click there.
            builder = builder.with_icon_templated(image);
        }
        #[cfg(not(target_os = "macos"))]
        {
            // Left-click toggles the shelf; the menu is on the right button.
            builder = builder.with_icon(image).with_menu_on_left_click(false);
        }

        let icon = builder.build().map_err(|e| e.to_string())?;
        Ok(Tray {
            _icon: icon,
            toggle,
            settings,
            quit,
        })
    }

    /// Relabel the menu after the language changed.
    pub fn set_language(&self, strings: &Strings) {
        self.toggle.set_text(strings.tray_toggle);
        self.settings.set_text(strings.tray_settings);
        self.quit.set_text(strings.tray_quit);
        let _ = self._icon.set_tooltip(Some(strings.tray_tooltip));
    }
}

fn load_icon(png: &[u8]) -> Result<Icon, String> {
    let image = image::load_from_memory(png)
        .map_err(|e| e.to_string())?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_icons_decode() {
        assert!(load_icon(APP_ICON).is_ok());
        assert!(load_icon(MENU_BAR_GLYPH).is_ok());
    }
}
