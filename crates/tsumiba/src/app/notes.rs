//! The notes column: a list on the left, an editor on the right.

use std::time::Instant;

use eframe::egui::{self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Sense, Ui, vec2};
use tsumiba_core::Note;

use super::theme::{self, Palette};
use super::widgets::{fitted, icon_button, primary_button, rule, section_title};
use super::{Action, fmt};
use crate::i18n::{Lang, Strings};

/// What the editor is showing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Selection {
    #[default]
    None,
    /// A note that does not exist yet; it is created by the first keystroke.
    Draft,
    Note(String),
}

#[derive(Debug, Default)]
pub struct NotesState {
    pub selected: Selection,
    /// What the editor currently contains.
    pub buffer: String,
    /// What the store holds for the selected note (to see whether `buffer` differs).
    pub loaded: String,
    /// When the buffer last changed and has not been saved yet.
    pub dirty_since: Option<Instant>,
    pub focus_editor: bool,
}

impl NotesState {
    pub fn start_draft(&mut self) {
        self.selected = Selection::Draft;
        self.buffer.clear();
        self.loaded.clear();
        self.dirty_since = None;
        self.focus_editor = true;
    }

    pub fn open(&mut self, note: &Note) {
        self.selected = Selection::Note(note.id.clone());
        self.buffer = note.body.clone();
        self.loaded = note.body.clone();
        self.dirty_since = None;
        self.focus_editor = true;
    }

    pub fn clear(&mut self) {
        self.selected = Selection::None;
        self.buffer.clear();
        self.loaded.clear();
        self.dirty_since = None;
    }

    pub fn selected_id(&self) -> Option<&str> {
        match &self.selected {
            Selection::Note(id) => Some(id),
            _ => None,
        }
    }

    /// The editor holds text that has not been saved.
    pub fn has_unsaved_text(&self) -> bool {
        self.dirty_since.is_some() || (self.selected == Selection::Draft && !self.buffer.is_empty())
    }
}

pub struct View<'a> {
    pub notes: &'a [Note],
    /// The note open in the editor, looked up among *all* notes so that a search
    /// that hides it from the list does not make the editor forget it.
    pub selected: Option<&'a Note>,
    pub total: usize,
    pub query: &'a str,
    pub pal: Palette,
    pub lang: Lang,
    pub s: &'static Strings,
    pub now_ms: u64,
}

pub const EDITOR_ID: &str = "tsumiba-note-editor";

pub fn show(ui: &mut Ui, state: &mut NotesState, view: &View, actions: &mut Vec<Action>) {
    let pal = view.pal;
    egui::Panel::left("note_list")
        .exact_size(214.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::NONE
                .fill(pal.surface)
                .inner_margin(Margin::symmetric(10, 10)),
        )
        .show(ui, |ui| list(ui, state, view, actions));

    egui::CentralPanel::default()
        .frame(
            Frame::NONE
                .fill(pal.raised)
                .inner_margin(Margin::symmetric(14, 10)),
        )
        .show(ui, |ui| editor(ui, state, view, actions));
}

fn list(ui: &mut Ui, state: &mut NotesState, view: &View, actions: &mut Vec<Action>) {
    let pal = view.pal;
    ui.horizontal(|ui| {
        section_title(
            ui,
            &pal,
            &view.s.count_notes(view.lang, view.total).to_uppercase(),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if primary_button(ui, &pal, "＋")
                .on_hover_text(view.s.new_note)
                .clicked()
            {
                actions.push(Action::NewNote);
            }
        });
    });
    ui.add_space(4.0);

    if view.notes.is_empty() {
        ui.add_space(20.0);
        let message = if view.query.trim().is_empty() {
            view.s.notes_empty
        } else {
            view.s.no_matches
        };
        ui.label(egui::RichText::new(message).color(pal.weak));
        return;
    }

    egui::ScrollArea::vertical()
        .id_salt("note_list_scroll")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for note in view.notes {
                let selected = state.selected_id() == Some(note.id.as_str());
                let response = row(ui, note, selected, view);
                if response.clicked() {
                    actions.push(Action::SelectNote(note.id.clone()));
                }
                response.context_menu(|ui| {
                    let pin = if note.pinned {
                        view.s.unpin
                    } else {
                        view.s.pin
                    };
                    if ui.button(pin).clicked() {
                        actions.push(Action::PinNote(note.id.clone(), !note.pinned));
                        ui.close();
                    }
                    if ui.button(view.s.copy_text).clicked() {
                        actions.push(Action::CopyText(note.body.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(view.s.delete).clicked() {
                        actions.push(Action::DeleteNote(note.id.clone()));
                        ui.close();
                    }
                });
            }
        });
}

fn row(ui: &mut Ui, note: &Note, selected: bool, view: &View) -> egui::Response {
    let pal = view.pal;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::click());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let background = if selected {
        theme::mix(pal.raised, pal.accent, 0.22)
    } else if response.hovered() {
        pal.raised
    } else {
        Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, background);
    if selected {
        let bar = egui::Rect::from_min_size(
            rect.left_top() + vec2(0.0, 10.0),
            vec2(3.0, rect.height() - 20.0),
        );
        painter.rect_filled(bar, 2.0, pal.accent);
    }

    let pad = 12.0;
    let time = fmt::relative_time(view.now_ms, note.updated_ms, view.lang);
    let time_galley = fitted(ui, &time, FontId::proportional(11.0), pal.weak, 60.0, 1);
    let time_pos = egui::pos2(rect.right() - pad - time_galley.size().x, rect.top() + 9.0);
    ui.painter().galley(time_pos, time_galley.clone(), pal.weak);

    let pin_width = if note.pinned { 16.0 } else { 0.0 };
    let title = note.title();
    let (title_text, title_colour) = if title.is_empty() {
        (view.s.untitled.to_string(), pal.weak)
    } else {
        (title, pal.text)
    };
    let title_width = rect.width() - pad * 2.0 - time_galley.size().x - 6.0 - pin_width;
    let title_galley = fitted(
        ui,
        &title_text,
        FontId::proportional(14.0),
        title_colour,
        title_width,
        1,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + pad + pin_width, rect.top() + 8.0),
        title_galley,
        title_colour,
    );
    if note.pinned {
        ui.painter().text(
            egui::pos2(rect.left() + pad, rect.top() + 16.0),
            Align2::LEFT_CENTER,
            "📌",
            FontId::proportional(11.0),
            pal.accent_ink,
        );
    }

    let snippet = note.snippet();
    if !snippet.is_empty() {
        let galley = fitted(
            ui,
            &snippet,
            FontId::proportional(12.0),
            pal.weak,
            rect.width() - pad * 2.0,
            1,
        );
        ui.painter().galley(
            egui::pos2(rect.left() + pad, rect.top() + 31.0),
            galley,
            pal.weak,
        );
    }
    response
}

fn editor(ui: &mut Ui, state: &mut NotesState, view: &View, actions: &mut Vec<Action>) {
    let pal = view.pal;
    let current_note = view.selected;

    // A slim toolbar above the text: only meaningful for a saved note.
    ui.horizontal(|ui| {
        match current_note {
            Some(note) => {
                let when = format!("{} {}", view.s.added, fmt::date(note.created_ms));
                ui.label(egui::RichText::new(when).small().color(pal.weak));
            }
            None => {
                ui.label(egui::RichText::new(view.s.new_note).small().color(pal.weak));
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(note) = current_note {
                if icon_button(ui, &pal, "🗑", view.s.delete, false).clicked() {
                    actions.push(Action::DeleteNote(note.id.clone()));
                }
                if icon_button(ui, &pal, super::widgets::COPY, view.s.copy_text, false).clicked() {
                    actions.push(Action::CopyText(note.body.clone()));
                }
                let pin = if note.pinned {
                    view.s.unpin
                } else {
                    view.s.pin
                };
                if icon_button(ui, &pal, "📌", pin, note.pinned).clicked() {
                    actions.push(Action::PinNote(note.id.clone(), !note.pinned));
                }
            }
        });
    });
    ui.add_space(2.0);
    rule(ui, &pal);
    ui.add_space(6.0);

    let editor_id = egui::Id::new(EDITOR_ID);
    let line = ui.text_style_height(&egui::TextStyle::Body);
    let rows = ((ui.available_height() / line).floor() as usize).max(4);

    egui::ScrollArea::vertical()
        .id_salt("note_editor_scroll")
        .auto_shrink(false)
        .show(ui, |ui| {
            let output = egui::TextEdit::multiline(&mut state.buffer)
                .id(editor_id)
                .frame(egui::Frame::NONE)
                .desired_width(f32::INFINITY)
                .desired_rows(rows)
                .lock_focus(true)
                .hint_text(egui::RichText::new(view.s.note_hint).color(pal.weak))
                .show(ui);
            if output.response.changed() {
                state.dirty_since = Some(Instant::now());
            }
            if state.focus_editor {
                output.response.request_focus();
                state.focus_editor = false;
            }
        });
}
