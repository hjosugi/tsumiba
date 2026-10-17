//! Small drawing helpers shared by the notes and files views.

use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, FontId, Galley, Response, RichText, Sense, Ui, Vec2, text::LayoutJob,
};

use super::theme::Palette;

/// Text laid out to fit `width` in at most `rows` lines, ending in `…` if cut.
pub fn fitted(
    ui: &Ui,
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    rows: usize,
) -> Arc<Galley> {
    let mut job = LayoutJob::simple(text.to_owned(), font, color, width.max(1.0));
    job.wrap.max_rows = rows;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    ui.painter().layout_job(job)
}

/// Pass this as the glyph to get a painted "copy" icon (two overlapping pages).
pub const COPY: &str = "copy";

fn paint_copy_icon(ui: &Ui, centre: egui::Pos2, colour: Color32) {
    let stroke = egui::Stroke::new(1.6, colour);
    let back = egui::Rect::from_center_size(centre + egui::vec2(2.0, -2.0), egui::vec2(10.0, 11.0));
    let front =
        egui::Rect::from_center_size(centre + egui::vec2(-2.0, 2.0), egui::vec2(10.0, 11.0));
    ui.painter()
        .rect_stroke(back, 2.0, stroke, egui::StrokeKind::Inside);
    // The front page hides the part of the back page that is behind it.
    ui.painter()
        .rect_filled(front, 2.0, ui.visuals().panel_fill);
    ui.painter()
        .rect_stroke(front, 2.0, stroke, egui::StrokeKind::Inside);
}

/// A square button showing one glyph, highlighted while `on`.
///
/// The button gets an id of its own, derived from what it is rather than from
/// how many widgets came before it: the header changes shape between screens
/// (the search box comes and goes) and positional ids would jump around.
pub fn icon_button(ui: &mut Ui, pal: &Palette, glyph: &str, tooltip: &str, on: bool) -> Response {
    ui.push_id((glyph, tooltip), |ui| {
        let size = Vec2::splat(28.0);
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        if ui.is_rect_visible(rect) {
            let background = if on {
                super::theme::mix(pal.raised, pal.accent, 0.30)
            } else if response.hovered() {
                pal.raised
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, 7.0, background);
            let colour = if on || response.hovered() {
                pal.text
            } else {
                pal.weak
            };
            if glyph == COPY {
                paint_copy_icon(ui, rect.center(), colour);
            } else {
                // The multiplication sign is the close mark; it needs to be drawn
                // larger than the other glyphs to look the same weight.
                let size = if glyph == "×" { 21.0 } else { 15.0 };
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    glyph,
                    FontId::proportional(size),
                    colour,
                );
            }
        }
        response.on_hover_text(tooltip)
    })
    .inner
}

/// The accent-coloured call-to-action button.
pub fn primary_button(ui: &mut Ui, pal: &Palette, label: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(pal.on_accent).strong())
            .fill(pal.accent)
            .stroke(egui::Stroke::NONE),
    )
}

/// A quiet section title.
pub fn section_title(ui: &mut Ui, pal: &Palette, text: &str) {
    ui.label(RichText::new(text).color(pal.weak).small().strong());
}

/// A thin horizontal rule in the border colour.
pub fn rule(ui: &mut Ui, pal: &Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, pal.border);
}
