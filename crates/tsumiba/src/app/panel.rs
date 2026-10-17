//! Drawing the window: header, columns, settings, and the overlays on top.

use eframe::egui::{
    self, Align, Align2, Frame, Id, Key, Layout, Margin, Order, RichText, Stroke, vec2,
};
use tsumiba_core::{Note, StoredFile, id};

use super::widgets::{icon_button, primary_button};
use super::{Action, Screen, TsumibaApp, files, notes, settings, theme};
use crate::i18n::{self, Strings};
use crate::platform::cursor;

const SEARCH_ID: &str = "tsumiba-search";

pub(super) fn draw(app: &mut TsumibaApp, ui: &mut egui::Ui) {
    let ctx = ui.ctx().clone();
    app.reconcile_caches();
    app.take_dropped_files(&ctx);

    let pal = theme::current(&ctx);
    let s = i18n::strings(app.lang);
    let mut actions: Vec<Action> = Vec::new();

    shortcuts(app, &ctx, &mut actions);

    egui::Panel::top("header")
        .exact_size(44.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::NONE
                .fill(pal.surface)
                .inner_margin(Margin::symmetric(14, 7)),
        )
        .show(ui, |ui| header(app, ui, &pal, s, &mut actions));

    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(pal.bg))
        .show(ui, |ui| match app.screen {
            Screen::Main => main_area(app, ui, &ctx, &pal, s, &mut actions),
            Screen::Settings => settings_area(app, ui, &ctx, &pal, s, &mut actions),
        });

    drop_overlay(&ctx, &pal, s);
    rename_modal(app, &ctx, &pal, s);
    toast(app, &ctx, &pal, s, &mut actions);

    // A hairline where the panel meets the desktop.
    let rect = ui.max_rect();
    ui.painter().hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        Stroke::new(1.0, pal.border),
    );

    app.apply(actions, &ctx);
}

fn shortcuts(app: &mut TsumibaApp, ctx: &egui::Context, actions: &mut Vec<Action>) {
    let popup_open = egui::Popup::is_any_open(ctx);
    let modal_open = app.files.rename.is_some() || app.settings.capturing_hotkey;

    ctx.input_mut(|input| {
        if input.consume_key(egui::Modifiers::COMMAND, Key::N) {
            actions.push(Action::NewNote);
        }
        if input.consume_key(egui::Modifiers::COMMAND, Key::Comma) {
            actions.push(Action::OpenSettings);
        }
        if input.consume_key(egui::Modifiers::COMMAND, Key::F) {
            ctx.memory_mut(|m| m.request_focus(Id::new(SEARCH_ID)));
        }
        if !popup_open && !modal_open && input.consume_key(egui::Modifiers::NONE, Key::Escape) {
            actions.push(match app.screen {
                Screen::Settings => Action::CloseSettings,
                Screen::Main => Action::Hide,
            });
        }
    });

    // Delete removes the selected tile, but never while a text field has the keys.
    if app.screen == Screen::Main
        && !modal_open
        && ctx.memory(|m| m.focused().is_none())
        && ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
        && let Some(id) = app.files.selected.clone()
    {
        actions.push(Action::RemoveFile(id));
    }
}

fn header(
    app: &mut TsumibaApp,
    ui: &mut egui::Ui,
    pal: &theme::Palette,
    s: &'static Strings,
    actions: &mut Vec<Action>,
) {
    ui.horizontal_centered(|ui| {
        ui.label(
            RichText::new("tsumiba")
                .color(pal.accent_ink)
                .strong()
                .size(15.0),
        );
        ui.add_space(10.0);

        if app.screen == Screen::Main {
            ui.add(
                egui::TextEdit::singleline(&mut app.search)
                    .id(Id::new(SEARCH_ID))
                    .hint_text(RichText::new(format!("🔍  {}", s.search_hint)).color(pal.weak))
                    .desired_width(230.0),
            );
            if !app.search.is_empty() && icon_button(ui, pal, "×", "", false).clicked() {
                app.search.clear();
            }
        }

        // An explicit id: the left half of the header changes between screens, and
        // an automatic id would shift with it.
        ui.push_id("header_actions", |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, pal, "×", s.hide, false).clicked() {
                    actions.push(Action::Hide);
                }
                if icon_button(ui, pal, "⚙", s.settings, app.screen == Screen::Settings).clicked()
                {
                    actions.push(if app.screen == Screen::Settings {
                        Action::CloseSettings
                    } else {
                        Action::OpenSettings
                    });
                }
                if icon_button(ui, pal, "📌", s.keep_open, app.keep_open).clicked() {
                    actions.push(Action::ToggleKeepOpen);
                }
            });
        });
    });
}

fn main_area(
    app: &mut TsumibaApp,
    ui: &mut egui::Ui,
    _ctx: &egui::Context,
    pal: &theme::Palette,
    s: &'static Strings,
    actions: &mut Vec<Action>,
) {
    let query = app.search.trim().to_string();
    let now_ms = id::now_ms();

    // Filter only when searching; otherwise borrow the cache as it is.
    let filtered_notes: Vec<Note>;
    let notes_view: &[Note] = if query.is_empty() {
        &app.caches.notes
    } else {
        filtered_notes = app
            .caches
            .notes
            .iter()
            .filter(|n| n.matches(&query))
            .cloned()
            .collect();
        &filtered_notes
    };
    let filtered_files: Vec<StoredFile>;
    let files_view: &[StoredFile] = if query.is_empty() {
        &app.caches.files
    } else {
        filtered_files = app
            .caches
            .files
            .iter()
            .filter(|f| f.matches(&query))
            .cloned()
            .collect();
        &filtered_files
    };

    let selected_note = app
        .notes
        .selected_id()
        .and_then(|id| app.caches.notes.iter().find(|n| n.id == id));
    let notes_input = notes::View {
        notes: notes_view,
        selected: selected_note,
        total: app.caches.notes.len(),
        query: &query,
        pal: *pal,
        lang: app.lang,
        s,
        now_ms,
    };
    let files_input = files::View {
        files: files_view,
        total: app.caches.files.len(),
        query: &query,
        pal: *pal,
        lang: app.lang,
        s,
        now_ms,
    };

    let (show_notes, show_files) = (app.cfg.panel.show_notes, app.cfg.panel.show_files);
    match (show_notes, show_files) {
        (true, true) => {
            let notes_width = (ui.available_width() * 0.56).round();
            egui::Panel::left("notes_column")
                .exact_size(notes_width)
                .resizable(false)
                .show_separator_line(true)
                .frame(Frame::NONE)
                .show(ui, |ui| {
                    notes::show(ui, &mut app.notes, &notes_input, actions)
                });
            egui::CentralPanel::default()
                .frame(Frame::NONE)
                .show(ui, |ui| {
                    files::show(ui, &mut app.files, &files_input, actions)
                });
        }
        (true, false) => notes::show(ui, &mut app.notes, &notes_input, actions),
        _ => files::show(ui, &mut app.files, &files_input, actions),
    }
}

fn settings_area(
    app: &mut TsumibaApp,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    pal: &theme::Palette,
    s: &'static Strings,
    actions: &mut Vec<Action>,
) {
    let view = settings::View {
        pal: *pal,
        s,
        token: &app.token,
        version: env!("CARGO_PKG_VERSION"),
        hotkey_available: app.hotkey.available(),
        edge_available: cursor::available(),
        api_port: app.server.as_ref().map(|server| server.port()),
    };
    let changed = settings::show(ui, &mut app.settings, &mut app.cfg, &view, actions);
    if changed {
        app.config_changed(ctx);
    }
}

fn drop_overlay(ctx: &egui::Context, pal: &theme::Palette, s: &'static Strings) {
    if ctx.input(|i| i.raw.hovered_files.is_empty()) {
        return;
    }
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        Order::Foreground,
        Id::new("drop_overlay"),
    ));
    painter.rect_filled(rect, 0.0, pal.bg.gamma_multiply(0.86));
    painter.rect_stroke(
        rect.shrink(8.0),
        14.0,
        Stroke::new(2.5, pal.accent),
        egui::StrokeKind::Inside,
    );
    painter.text(
        rect.center() - vec2(0.0, 14.0),
        Align2::CENTER_CENTER,
        "⬇",
        egui::FontId::proportional(34.0),
        pal.accent,
    );
    painter.text(
        rect.center() + vec2(0.0, 22.0),
        Align2::CENTER_CENTER,
        s.drop_to_add,
        egui::FontId::proportional(16.0),
        pal.text,
    );
}

fn rename_modal(
    app: &mut TsumibaApp,
    ctx: &egui::Context,
    pal: &theme::Palette,
    s: &'static Strings,
) {
    let Some(rename) = app.files.rename.as_mut() else {
        return;
    };
    let mut confirm = false;
    let mut cancel = false;
    let response = egui::Modal::new(Id::new("rename_modal")).show(ctx, |ui| {
        ui.set_width(340.0);
        ui.heading(s.rename);
        ui.add_space(6.0);
        let edit =
            ui.add(egui::TextEdit::singleline(&mut rename.text).desired_width(f32::INFINITY));
        if rename.focus {
            edit.request_focus();
            rename.focus = false;
        }
        if edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            confirm = true;
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if primary_button(ui, pal, s.rename).clicked() {
                confirm = true;
            }
            if ui.button(s.cancel).clicked() {
                cancel = true;
            }
        });
    });
    if response.should_close() {
        cancel = true;
    }
    if confirm {
        if let Some(done) = app.files.rename.take()
            && let Err(e) = app.store.rename_file(&done.id, &done.text)
        {
            app.set_toast(e.to_string(), None);
        }
    } else if cancel {
        app.files.rename = None;
    }
}

fn toast(
    app: &mut TsumibaApp,
    ctx: &egui::Context,
    pal: &theme::Palette,
    s: &'static Strings,
    actions: &mut Vec<Action>,
) {
    let Some(toast) = &app.toast else {
        return;
    };
    egui::Area::new(Id::new("toast"))
        .order(Order::Tooltip)
        .anchor(Align2::LEFT_BOTTOM, vec2(16.0, -14.0))
        .show(ctx, |ui| {
            Frame::popup(ui.style())
                .fill(pal.raised)
                .stroke(Stroke::new(1.0, pal.border))
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(14, 9))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(&toast.text);
                        if toast.undo.is_some()
                            && ui
                                .button(RichText::new(s.undo).color(pal.accent_ink).strong())
                                .clicked()
                        {
                            actions.push(Action::Undo);
                        }
                        if icon_button(ui, pal, "×", "", false).clicked() {
                            actions.push(Action::DismissToast);
                        }
                    });
                });
        });
}
