//! The files column: a wall of tiles you can drop things onto.

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, Sense, Stroke, Ui, vec2,
};
use tsumiba_core::StoredFile;

use super::theme::{self, Palette};
use super::widgets::{fitted, primary_button, section_title};
use super::{Action, fmt};
use crate::i18n::{Lang, Strings};

const TILE_W: f32 = 94.0;
const ICON_H: f32 = 66.0;
const TILE_H: f32 = 124.0;
/// Larger images show a badge instead of a thumbnail: decoding a 50-megapixel
/// photo just to draw 64 pixels of it would stall the whole panel.
const MAX_THUMBNAIL_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct FilesState {
    pub selected: Option<String>,
    pub rename: Option<RenameState>,
}

#[derive(Debug)]
pub struct RenameState {
    pub id: String,
    pub text: String,
    pub focus: bool,
}

pub struct View<'a> {
    pub files: &'a [StoredFile],
    pub total: usize,
    pub query: &'a str,
    pub pal: Palette,
    pub lang: Lang,
    pub s: &'static Strings,
    pub now_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image,
    Document,
    Archive,
    Media,
    Code,
    Folder,
    Other,
}

pub fn kind_of(file: &StoredFile) -> Kind {
    if file.is_dir {
        return Kind::Folder;
    }
    match file.extension().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "avif" | "heic" | "tiff"
        | "ico" => Kind::Image,
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp"
        | "txt" | "md" | "rtf" | "csv" | "epub" | "pages" | "numbers" | "key" => Kind::Document,
        "zip" | "tar" | "gz" | "tgz" | "xz" | "bz2" | "7z" | "rar" | "zst" | "dmg" | "iso"
        | "deb" | "rpm" => Kind::Archive,
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "mp4" | "mkv" | "mov" | "avi" | "webm"
        | "opus" => Kind::Media,
        "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "c" | "h" | "cpp" | "hpp" | "java"
        | "kt" | "rb" | "php" | "sh" | "json" | "toml" | "yaml" | "yml" | "html" | "css"
        | "sql" | "lua" | "nix" => Kind::Code,
        _ => Kind::Other,
    }
}

/// Badge colours. They are deep enough for white text (4.5:1 or better) and are
/// the same in both themes, so a file type looks the same day and night.
fn kind_colour(kind: Kind, pal: &Palette) -> Color32 {
    match kind {
        Kind::Image => Color32::from_rgb(0x1f, 0x7d, 0x71),
        Kind::Document => Color32::from_rgb(0x3a, 0x67, 0xc2),
        Kind::Archive => Color32::from_rgb(0x8a, 0x5a, 0x2b),
        Kind::Media => Color32::from_rgb(0x74, 0x44, 0xb3),
        Kind::Code => Color32::from_rgb(0x3a, 0x7d, 0x3e),
        Kind::Folder => pal.accent,
        Kind::Other => Color32::from_rgb(0x66, 0x70, 0x8a),
    }
}

/// Paint an image filling `rect`, cropped to fit rather than squashed.
fn paint_thumbnail(ui: &Ui, uri: String, rect: egui::Rect) {
    let image = egui::Image::new(uri)
        .corner_radius(8.0)
        .show_loading_spinner(false);
    let uv = image
        .load_for_size(ui.ctx(), rect.size())
        .ok()
        .and_then(|poll| poll.size())
        .map(|size| cover_uv(size, rect.size()))
        .unwrap_or_else(|| egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)));
    image.uv(uv).paint_at(ui, rect);
}

/// The part of a `source`-sized image (as 0..1 texture coordinates) that fills a
/// `target`-shaped box with no distortion: the middle, cropped on the long side.
pub fn cover_uv(source: egui::Vec2, target: egui::Vec2) -> egui::Rect {
    let full = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    if source.x <= 0.0 || source.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return full;
    }
    let source_aspect = source.x / source.y;
    let target_aspect = target.x / target.y;
    if source_aspect > target_aspect {
        // Too wide: crop the sides.
        let visible = target_aspect / source_aspect;
        let margin = (1.0 - visible) / 2.0;
        egui::Rect::from_min_max(egui::pos2(margin, 0.0), egui::pos2(1.0 - margin, 1.0))
    } else {
        // Too tall: crop top and bottom.
        let visible = source_aspect / target_aspect;
        let margin = (1.0 - visible) / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, margin), egui::pos2(1.0, 1.0 - margin))
    }
}

/// The file:// URI egui should load a thumbnail from, when one is sensible.
pub fn thumbnail_uri(file: &StoredFile) -> Option<String> {
    let raster = matches!(
        file.extension().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp"
    );
    if !raster || file.is_dir || file.missing || file.size > MAX_THUMBNAIL_BYTES {
        return None;
    }
    Some(file_uri(&file.path.to_string_lossy(), cfg!(windows)))
}

/// The `file://` URI that egui's file loader turns back into `path`.
///
/// The loader does no percent-decoding, so the path goes in as it is. What differs
/// is the slashes: on Unix the path's own leading `/` supplies the third slash of
/// `file:///home/me/a.png`; on Windows the loader reads `file://C:\a.png` as a UNC
/// host and needs the explicit slash of `file:///C:\a.png` to know it is a drive.
pub fn file_uri(path: &str, windows: bool) -> String {
    if windows {
        format!("file:///{path}")
    } else {
        format!("file://{path}")
    }
}

pub fn show(ui: &mut Ui, state: &mut FilesState, view: &View, actions: &mut Vec<Action>) {
    let pal = view.pal;
    Frame::NONE
        .fill(pal.bg)
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                section_title(
                    ui,
                    &pal,
                    &view.s.count_files(view.lang, view.total).to_uppercase(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, &pal, view.s.add_files).clicked() {
                        actions.push(Action::AddFilesDialog);
                    }
                });
            });
            ui.add_space(6.0);

            if view.files.is_empty() {
                empty(ui, view);
                return;
            }

            egui::ScrollArea::vertical()
                .id_salt("files_scroll")
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    // Top-aligned, so every tile in a row starts at the same height.
                    ui.with_layout(
                        Layout::left_to_right(Align::Min).with_main_wrap(true),
                        |ui| {
                            for file in view.files {
                                tile(ui, state, file, view, actions);
                            }
                        },
                    );
                });
        });
}

fn empty(ui: &mut Ui, view: &View) {
    let pal = view.pal;
    let rect = ui.available_rect_before_wrap().shrink2(vec2(2.0, 8.0));
    let message = if view.query.trim().is_empty() {
        view.s.files_empty
    } else {
        view.s.no_matches
    };
    ui.painter().rect_stroke(
        rect,
        12.0,
        Stroke::new(1.5, pal.border),
        egui::StrokeKind::Inside,
    );
    let searching = !view.query.trim().is_empty();
    if !searching {
        ui.painter().text(
            rect.center() - vec2(0.0, 10.0),
            Align2::CENTER_CENTER,
            "⬇",
            FontId::proportional(26.0),
            pal.weak,
        );
    }
    ui.painter().text(
        rect.center() + vec2(0.0, if searching { 0.0 } else { 20.0 }),
        Align2::CENTER_CENTER,
        message,
        FontId::proportional(13.5),
        pal.weak,
    );
}

fn tile(
    ui: &mut Ui,
    state: &mut FilesState,
    file: &StoredFile,
    view: &View,
    actions: &mut Vec<Action>,
) {
    let pal = view.pal;
    let (rect, response) = ui.allocate_exact_size(vec2(TILE_W, TILE_H), Sense::click());
    if !ui.is_rect_visible(rect) {
        return;
    }
    let selected = state.selected.as_deref() == Some(file.id.as_str());

    let background = if selected {
        theme::mix(pal.raised, pal.accent, 0.20)
    } else if response.hovered() {
        pal.raised
    } else {
        pal.surface
    };
    ui.painter().rect_filled(rect, 10.0, background);
    if selected {
        ui.painter().rect_stroke(
            rect,
            10.0,
            Stroke::new(1.5, pal.accent),
            egui::StrokeKind::Inside,
        );
    }

    // Icon area.
    let icon_rect =
        egui::Rect::from_min_size(rect.min + vec2(5.0, 5.0), vec2(TILE_W - 10.0, ICON_H));
    let kind = kind_of(file);
    ui.painter()
        .rect_filled(icon_rect, 8.0, kind_colour(kind, &pal));
    if let Some(uri) = thumbnail_uri(file) {
        paint_thumbnail(ui, uri, icon_rect);
    } else {
        let label = match kind {
            Kind::Folder => "📁".to_string(),
            _ => {
                let ext = file.extension();
                if ext.is_empty() {
                    "FILE".to_string()
                } else {
                    ext.chars().take(4).collect::<String>().to_uppercase()
                }
            }
        };
        let font = if kind == Kind::Folder {
            FontId::proportional(28.0)
        } else {
            FontId::proportional(16.0)
        };
        ui.painter().text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            label,
            font,
            Color32::WHITE,
        );
    }
    if file.missing {
        ui.painter()
            .rect_filled(icon_rect, 8.0, Color32::from_black_alpha(150));
        ui.painter().text(
            icon_rect.center(),
            Align2::CENTER_CENTER,
            view.s.missing,
            FontId::proportional(12.0),
            pal.danger,
        );
    }
    if file.pinned {
        ui.painter().text(
            icon_rect.left_top() + vec2(5.0, 4.0),
            Align2::LEFT_TOP,
            "📌",
            FontId::proportional(11.0),
            Color32::WHITE,
        );
    }
    if file.link && !file.missing {
        ui.painter().text(
            icon_rect.right_top() + vec2(-5.0, 3.0),
            Align2::RIGHT_TOP,
            "↗",
            FontId::proportional(12.0),
            Color32::WHITE,
        );
    }

    // Name and size.
    let text_w = TILE_W - 12.0;
    let name = fitted(
        ui,
        &file.name,
        FontId::proportional(12.0),
        pal.text,
        text_w,
        2,
    );
    ui.painter().galley(
        egui::pos2(rect.left() + 6.0, icon_rect.bottom() + 5.0),
        name.clone(),
        pal.text,
    );
    let meta = if file.is_dir {
        view.s.folder.to_string()
    } else {
        fmt::size(file.size)
    };
    let meta = fitted(ui, &meta, FontId::proportional(10.5), pal.weak, text_w, 1);
    ui.painter().galley(
        egui::pos2(rect.left() + 6.0, rect.bottom() - 17.0),
        meta,
        pal.weak,
    );

    if response.clicked() {
        state.selected = Some(file.id.clone());
    }
    if response.double_clicked() {
        actions.push(Action::OpenFile(file.id.clone()));
    }

    let mut tip = format!(
        "{}\n{} · {}",
        file.name,
        if file.is_dir {
            view.s.folder.to_string()
        } else {
            fmt::size(file.size)
        },
        fmt::relative_time(view.now_ms, file.added_ms, view.lang),
    );
    if let Some(source) = &file.source {
        tip.push_str(&format!("\n{}: {}", view.s.from, source));
    }
    if file.link {
        tip.push_str(&format!("\n{}: {}", view.s.linked, file.path.display()));
    }
    let response = response.on_hover_text(tip);

    response.context_menu(|ui| {
        let id = file.id.clone();
        let mut item = |ui: &mut Ui, label: &str, action: Action| {
            if ui.button(label).clicked() {
                actions.push(action);
                ui.close();
            }
        };
        item(ui, view.s.open, Action::OpenFile(id.clone()));
        item(ui, view.s.show_in_folder, Action::RevealFile(id.clone()));
        ui.separator();
        if !file.is_dir {
            item(ui, view.s.copy_file, Action::CopyFile(id.clone()));
        }
        item(ui, view.s.copy_path, Action::CopyPath(id.clone()));
        if !file.is_dir {
            item(ui, view.s.save_copy, Action::SaveCopy(id.clone()));
        }
        item(ui, view.s.rename, Action::StartRename(id.clone()));
        let pin = if file.pinned {
            view.s.unpin
        } else {
            view.s.pin
        };
        item(ui, pin, Action::PinFile(id.clone(), !file.pinned));
        ui.separator();
        item(ui, view.s.remove, Action::RemoveFile(id));
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn file(name: &str, size: u64) -> StoredFile {
        StoredFile {
            id: "0".repeat(20),
            name: name.to_string(),
            size,
            added_ms: 0,
            pinned: false,
            source: None,
            path: PathBuf::from("/tmp").join(name),
            link: false,
            is_dir: false,
            missing: false,
        }
    }

    #[test]
    fn cover_crops_the_long_side_and_never_distorts() {
        let square = egui::vec2(80.0, 80.0);
        // Already the right shape: the whole image.
        assert_eq!(
            cover_uv(egui::vec2(100.0, 100.0), square),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0))
        );
        // Wide image: the middle half of the width.
        let wide = cover_uv(egui::vec2(200.0, 100.0), square);
        assert_eq!(
            (wide.min.x, wide.max.x, wide.min.y, wide.max.y),
            (0.25, 0.75, 0.0, 1.0)
        );
        // Tall image: the middle half of the height.
        let tall = cover_uv(egui::vec2(100.0, 200.0), square);
        assert_eq!(
            (tall.min.y, tall.max.y, tall.min.x, tall.max.x),
            (0.25, 0.75, 0.0, 1.0)
        );
        // Garbage in, whole image out.
        let all = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        assert_eq!(cover_uv(egui::vec2(0.0, 0.0), square), all);
        assert_eq!(cover_uv(egui::vec2(10.0, 10.0), egui::vec2(0.0, 5.0)), all);
    }

    #[test]
    fn kinds_by_extension() {
        assert_eq!(kind_of(&file("a.PNG", 1)), Kind::Image);
        assert_eq!(kind_of(&file("a.pdf", 1)), Kind::Document);
        assert_eq!(kind_of(&file("a.tar", 1)), Kind::Archive);
        assert_eq!(kind_of(&file("a.mp4", 1)), Kind::Media);
        assert_eq!(kind_of(&file("main.rs", 1)), Kind::Code);
        assert_eq!(kind_of(&file("README", 1)), Kind::Other);
        let mut folder = file("project", 0);
        folder.is_dir = true;
        assert_eq!(kind_of(&folder), Kind::Folder);
    }

    #[test]
    fn only_small_raster_images_get_thumbnails() {
        assert!(thumbnail_uri(&file("a.png", 1024)).is_some());
        assert!(thumbnail_uri(&file("a.jpeg", 1024)).is_some());
        assert!(
            thumbnail_uri(&file("a.svg", 1024)).is_none(),
            "vector images are not decoded"
        );
        assert!(thumbnail_uri(&file("a.pdf", 1024)).is_none());
        assert!(thumbnail_uri(&file("huge.png", MAX_THUMBNAIL_BYTES + 1)).is_none());
        let mut gone = file("a.png", 1);
        gone.missing = true;
        assert!(thumbnail_uri(&gone).is_none());
    }

    #[test]
    fn file_uris_are_shaped_the_way_the_loader_expects_on_each_os() {
        assert_eq!(
            file_uri("/home/me/my pic.png", false),
            "file:///home/me/my pic.png"
        );
        assert_eq!(
            file_uri("C:\\Users\\me\\a.png", true),
            "file:///C:\\Users\\me\\a.png"
        );
        // A UNC share keeps its two backslashes after the explicit slash.
        assert_eq!(
            file_uri("\\\\nas\\share\\a.png", true),
            "file:///\\\\nas\\share\\a.png"
        );
    }

    #[cfg(unix)]
    #[test]
    fn thumbnail_uri_names_the_stored_path() {
        let uri = thumbnail_uri(&file("a.png", 1)).unwrap();
        assert_eq!(uri, "file:///tmp/a.png");
    }
}
