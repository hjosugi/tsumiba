//! The settings screen.

use eframe::egui::{self, Event, Frame, Margin, RichText, Ui, vec2};
use tsumiba_core::Config;
use tsumiba_core::config::{Language, StorageMode, Theme};

use super::Action;
use super::theme::Palette;
use super::widgets::{icon_button, section_title};
use crate::i18n::Strings;
use crate::platform::hotkey;

#[derive(Debug, Default)]
pub struct SettingsState {
    /// Waiting for the user to press the new shortcut.
    pub capturing_hotkey: bool,
    pub hotkey_error: Option<String>,
    pub port_text: String,
    pub token_revealed: bool,
}

pub struct View<'a> {
    pub pal: Palette,
    pub s: &'static Strings,
    pub token: &'a str,
    pub version: &'a str,
    pub hotkey_available: bool,
    pub edge_available: bool,
    /// The port the API is actually listening on (differs from the setting if it was taken).
    pub api_port: Option<u16>,
}

/// Draw the screen. Returns whether any setting in `config` changed.
pub fn show(
    ui: &mut Ui,
    state: &mut SettingsState,
    config: &mut Config,
    view: &View,
    actions: &mut Vec<Action>,
) -> bool {
    let pal = view.pal;
    let s = view.s;
    let mut changed = false;

    Frame::NONE
        .fill(pal.surface)
        .inner_margin(Margin::symmetric(18, 12))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                if ui.button(format!("‹  {}", s.back)).clicked() {
                    actions.push(Action::CloseSettings);
                }
                ui.add_space(8.0);
                ui.label(RichText::new(s.settings).heading());
            });
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .auto_shrink(false)
                .show(ui, |ui| {
                    // The panel is wide and short: lay the groups out side by side.
                    ui.columns(3, |columns| {
                        changed |= general(&mut columns[0], config, view, actions);
                        changed |= panel(&mut columns[0], config, view);
                        changed |= summon(&mut columns[1], state, config, view, actions);
                        changed |= storage(&mut columns[1], config, view);
                        changed |= browser(&mut columns[2], state, config, view, actions);
                        about(&mut columns[2], view, actions);
                    });
                });
        });
    changed
}

fn group<R>(ui: &mut Ui, pal: &Palette, title: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.add_space(10.0);
    section_title(ui, pal, &title.to_uppercase());
    ui.add_space(2.0);
    add(ui)
}

fn general(ui: &mut Ui, config: &mut Config, view: &View, actions: &mut Vec<Action>) -> bool {
    let s = view.s;
    let mut changed = false;
    group(ui, &view.pal, s.general, |ui| {
        egui::Grid::new("general")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(s.language);
                let label = match config.general.language {
                    Language::Auto => s.language_auto,
                    Language::En => "English",
                    Language::Ja => "日本語",
                };
                egui::ComboBox::from_id_salt("language")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut config.general.language,
                                Language::Auto,
                                s.language_auto,
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(&mut config.general.language, Language::En, "English")
                            .changed();
                        changed |= ui
                            .selectable_value(&mut config.general.language, Language::Ja, "日本語")
                            .changed();
                    });
                ui.end_row();

                ui.label(s.theme);
                let label = match config.general.theme {
                    Theme::System => s.theme_system,
                    Theme::Light => s.theme_light,
                    Theme::Dark => s.theme_dark,
                };
                egui::ComboBox::from_id_salt("theme")
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        changed |= ui
                            .selectable_value(
                                &mut config.general.theme,
                                Theme::System,
                                s.theme_system,
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(
                                &mut config.general.theme,
                                Theme::Light,
                                s.theme_light,
                            )
                            .changed();
                        changed |= ui
                            .selectable_value(&mut config.general.theme, Theme::Dark, s.theme_dark)
                            .changed();
                    });
                ui.end_row();
            });
        if ui
            .checkbox(&mut config.general.launch_at_login, s.launch_at_login)
            .changed()
        {
            changed = true;
            actions.push(Action::ApplyAutostart);
        }
    });
    changed
}

fn panel(ui: &mut Ui, config: &mut Config, view: &View) -> bool {
    let s = view.s;
    let mut changed = false;
    group(ui, &view.pal, s.panel, |ui| {
        ui.horizontal(|ui| {
            changed |= ui
                .checkbox(&mut config.panel.show_notes, s.show_notes)
                .changed();
            ui.add_space(12.0);
            changed |= ui
                .checkbox(&mut config.panel.show_files, s.show_files)
                .changed();
        });
        egui::Grid::new("panel")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(s.width);
                changed |= ui
                    .add(egui::Slider::new(&mut config.panel.width, 480.0..=1600.0).suffix(" pt"))
                    .changed();
                ui.end_row();
                ui.label(s.height);
                changed |= ui
                    .add(egui::Slider::new(&mut config.panel.height, 240.0..=900.0).suffix(" pt"))
                    .changed();
                ui.end_row();
            });
        changed |= ui
            .checkbox(&mut config.panel.auto_hide, s.auto_hide)
            .changed();
    });
    changed
}

fn summon(
    ui: &mut Ui,
    state: &mut SettingsState,
    config: &mut Config,
    view: &View,
    actions: &mut Vec<Action>,
) -> bool {
    let s = view.s;
    let pal = view.pal;
    let mut changed = false;
    group(ui, &pal, s.summon, |ui| {
        ui.horizontal(|ui| {
            ui.label(s.hotkey);
            ui.add_space(8.0);
            let label = if state.capturing_hotkey {
                s.hotkey_listening.to_string()
            } else if config.trigger.hotkey.is_empty() {
                s.hotkey_off.to_string()
            } else {
                config.trigger.hotkey.clone()
            };
            let button = ui.add_enabled(
                view.hotkey_available,
                egui::Button::new(RichText::new(label).monospace()).min_size(vec2(180.0, 0.0)),
            );
            if button.clicked() {
                state.capturing_hotkey = true;
                state.hotkey_error = None;
            }
            button.on_hover_text(s.hotkey_press);
            if view.hotkey_available
                && !config.trigger.hotkey.is_empty()
                && icon_button(ui, &pal, "×", s.hotkey_off, false).clicked()
            {
                config.trigger.hotkey.clear();
                state.capturing_hotkey = false;
                changed = true;
                actions.push(Action::ApplyHotkey);
            }
        });
        if !view.hotkey_available {
            ui.label(RichText::new(s.hotkey_unavailable).small().color(pal.weak));
        }
        if let Some(error) = &state.hotkey_error {
            ui.label(RichText::new(error).small().color(pal.danger));
        }
        if state.capturing_hotkey {
            capture_hotkey(ui, state, config, s, &mut changed, actions);
        }

        ui.add_space(6.0);
        ui.add_enabled_ui(view.edge_available, |ui| {
            changed |= ui.checkbox(&mut config.trigger.edge, s.edge).changed();
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(s.edge_dwell);
                changed |= ui
                    .add(egui::Slider::new(
                        &mut config.trigger.edge_dwell_ms,
                        0..=1500,
                    ))
                    .changed();
            });
        });
        if !view.edge_available {
            ui.label(RichText::new(s.edge_unavailable).small().color(pal.weak));
        }
    });
    changed
}

fn capture_hotkey(
    ui: &mut Ui,
    state: &mut SettingsState,
    config: &mut Config,
    s: &Strings,
    changed: &mut bool,
    actions: &mut Vec<Action>,
) {
    let events = ui.input(|i| i.events.clone());
    for event in events {
        let Event::Key {
            key,
            pressed: true,
            modifiers,
            repeat: false,
            ..
        } = event
        else {
            continue;
        };
        if key == egui::Key::Escape && modifiers.is_none() {
            state.capturing_hotkey = false;
            return;
        }
        match hotkey::describe(modifiers, key) {
            Some(spec) => {
                config.trigger.hotkey = spec;
                state.capturing_hotkey = false;
                state.hotkey_error = None;
                *changed = true;
                actions.push(Action::ApplyHotkey);
            }
            None => state.hotkey_error = Some(s.hotkey_invalid.to_string()),
        }
        return;
    }
}

fn storage(ui: &mut Ui, config: &mut Config, view: &View) -> bool {
    let s = view.s;
    let mut changed = false;
    group(ui, &view.pal, s.storage, |ui| {
        changed |= ui
            .radio_value(&mut config.storage.mode, StorageMode::Copy, s.store_copy)
            .changed();
        changed |= ui
            .radio_value(&mut config.storage.mode, StorageMode::Link, s.store_link)
            .changed();
        ui.label(RichText::new(s.store_hint).small().color(view.pal.weak));
    });
    changed
}

fn browser(
    ui: &mut Ui,
    state: &mut SettingsState,
    config: &mut Config,
    view: &View,
    actions: &mut Vec<Action>,
) -> bool {
    let s = view.s;
    let pal = view.pal;
    let mut changed = false;
    group(ui, &pal, s.browser, |ui| {
        changed |= ui
            .checkbox(&mut config.api.enabled, s.api_enabled)
            .changed();
        ui.label(RichText::new(s.restart_note).small().color(pal.weak));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(s.api_port);
            if state.port_text.is_empty() {
                state.port_text = config.api.port.to_string();
            }
            let response =
                ui.add(egui::TextEdit::singleline(&mut state.port_text).desired_width(64.0));
            if response.lost_focus() {
                match state.port_text.trim().parse::<u16>() {
                    Ok(port) if port == 0 || port >= 1024 => {
                        if config.api.port != port {
                            config.api.port = port;
                            changed = true;
                        }
                    }
                    _ => state.port_text = config.api.port.to_string(),
                }
            }
            if let Some(actual) = view.api_port
                && actual != config.api.port
            {
                ui.label(RichText::new(format!("→ {actual}")).small().color(pal.weak));
            }
        });

        ui.add_space(2.0);
        ui.label(s.api_token);
        let shown = if state.token_revealed {
            view.token.to_string()
        } else {
            format!(
                "{}{}",
                &view.token[..view.token.len().min(6)],
                "•".repeat(14)
            )
        };
        // Click the token to reveal or hide it.
        let token_label = egui::Label::new(RichText::new(shown).monospace().size(12.5))
            .sense(egui::Sense::click());
        if ui.add(token_label).clicked() {
            state.token_revealed = !state.token_revealed;
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button(s.copy).clicked() {
                actions.push(Action::CopyToken);
            }
            if ui
                .button(s.regenerate)
                .on_hover_text(s.regenerate_hint)
                .clicked()
            {
                actions.push(Action::RegenerateToken);
            }
        });
    });
    changed
}

fn about(ui: &mut Ui, view: &View, actions: &mut Vec<Action>) {
    let s = view.s;
    group(ui, &view.pal, s.about, |ui| {
        ui.label(
            RichText::new(format!("tsumiba  ·  {} {}", s.version, view.version))
                .color(view.pal.weak),
        );
        ui.horizontal(|ui| {
            if ui.button(s.open_data_folder).clicked() {
                actions.push(Action::OpenDataFolder);
            }
            if ui.button(s.open_config_file).clicked() {
                actions.push(Action::OpenConfigFile);
            }
        });
    });
}
