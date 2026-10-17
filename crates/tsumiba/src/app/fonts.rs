//! Japanese (and other CJK) text.
//!
//! egui's bundled fonts have no CJK glyphs, so without help every Japanese
//! character in a note would be drawn as an empty box. Rather than ship a
//! multi-megabyte font, tsumiba borrows one the system already has: it looks for
//! a well-known CJK family, checks that the font really contains Japanese
//! characters, and adds it to egui as a fallback behind the Latin fonts.
//!
//! Scanning the system's fonts takes a moment, so it happens on a background
//! thread and the text re-renders once the font is in.

use std::sync::Arc;

use eframe::egui;
use fontdb::{Database, Family, Query, Stretch, Style, Weight};

/// Families to try, best first. Japanese-specific faces come first so that
/// shared Han characters are drawn with Japanese glyph shapes.
const CANDIDATES: &[&str] = &[
    // macOS
    "Hiragino Sans",
    "Hiragino Kaku Gothic ProN",
    "Hiragino Kaku Gothic Pro",
    // Windows
    "Yu Gothic UI",
    "Yu Gothic",
    "Meiryo UI",
    "Meiryo",
    "MS UI Gothic",
    // Linux
    "Noto Sans CJK JP",
    "Noto Sans JP",
    "Source Han Sans JP",
    "Source Han Sans",
    "IPAexGothic",
    "IPAGothic",
    "IPAPGothic",
    "TakaoPGothic",
    "VL PGothic",
    "M PLUS 1p",
    "BIZ UDPGothic",
    // Other CJK, so Chinese and Korean text is at least readable
    "Noto Sans CJK SC",
    "Noto Sans CJK KR",
    "PingFang SC",
    "Microsoft YaHei UI",
    "Malgun Gothic",
    "WenQuanYi Micro Hei",
    "Droid Sans Fallback",
];

/// Characters a usable Japanese font must contain.
const PROBES: &[char] = &['あ', 'ア', '漢', '日'];

/// Find a CJK font on a background thread and install it into `ctx`.
pub fn install_in_background(ctx: egui::Context) {
    let spawned = std::thread::Builder::new()
        .name("tsumiba-fonts".into())
        .spawn(move || match find_cjk_font() {
            Some(font) => {
                log::info!("using {} for Japanese text", font.family);
                let mut definitions = egui::FontDefinitions::default();
                let mut data = egui::FontData::from_owned(font.bytes);
                data.index = font.index;
                definitions
                    .font_data
                    .insert("system-cjk".to_string(), Arc::new(data));
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    definitions
                        .families
                        .entry(family)
                        .or_default()
                        .push("system-cjk".to_string());
                }
                ctx.set_fonts(definitions);
                ctx.request_repaint();
            }
            None => log::warn!("no system font with Japanese glyphs was found"),
        });
    if let Err(e) = spawned {
        log::warn!("could not start the font loader: {e}");
    }
}

struct Found {
    family: String,
    bytes: Vec<u8>,
    index: u32,
}

fn find_cjk_font() -> Option<Found> {
    let mut db = Database::new();
    db.load_system_fonts();

    // First: the families we know, in order of preference.
    for name in CANDIDATES {
        let query = Query {
            families: &[Family::Name(name)],
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };
        if let Some(id) = db.query(&query)
            && let Some(found) = read_if_japanese(&db, id, name)
        {
            return Some(found);
        }
    }

    // Otherwise: anything that looks like a CJK face by name and has the glyphs.
    let ids: Vec<_> = db
        .faces()
        .filter(|face| {
            face.families.iter().any(|(family, _)| {
                let f = family.to_ascii_lowercase();
                f.contains("cjk")
                    || f.contains("gothic")
                    || f.contains("mincho")
                    || f.contains("jp")
            })
        })
        .map(|face| (face.id, face.families.first().map(|(n, _)| n.clone())))
        .collect();
    for (id, name) in ids {
        if let Some(found) = read_if_japanese(&db, id, name.as_deref().unwrap_or("unknown")) {
            return Some(found);
        }
    }
    None
}

fn read_if_japanese(db: &Database, id: fontdb::ID, family: &str) -> Option<Found> {
    db.with_face_data(id, |data, index| {
        let face = ttf_parser::Face::parse(data, index).ok()?;
        if PROBES.iter().all(|&c| face.glyph_index(c).is_some()) {
            Some(Found {
                family: family.to_string(),
                bytes: data.to_vec(),
                index,
            })
        } else {
            None
        }
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_list_has_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for name in CANDIDATES {
            assert!(seen.insert(*name), "{name} is listed twice");
        }
    }

    #[test]
    fn japanese_faces_come_before_other_cjk() {
        let jp = CANDIDATES
            .iter()
            .position(|n| *n == "Noto Sans CJK JP")
            .unwrap();
        let sc = CANDIDATES
            .iter()
            .position(|n| *n == "Noto Sans CJK SC")
            .unwrap();
        assert!(jp < sc);
    }
}
