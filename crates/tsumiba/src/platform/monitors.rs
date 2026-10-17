//! Where the screens are.
//!
//! Coordinates here are *native*: physical pixels on Windows and X11, points on
//! macOS. They are the same space the pointer position is reported in, which is
//! what makes "is the pointer on this monitor's top edge" a plain comparison.
//! The window is positioned in egui *points*, so [`Monitor::native_per_point`]
//! converts between the two.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Monitor {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// Pixels per point as the platform reports it (1.0 when unknown).
    pub scale: f32,
    pub primary: bool,
}

impl Monitor {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && y >= self.y
            && (x as i64) < self.x as i64 + self.width as i64
            && (y as i64) < self.y as i64 + self.height as i64
    }

    /// Native units per egui point on this monitor.
    pub fn native_per_point(&self) -> f32 {
        if cfg!(target_os = "macos") {
            // Coordinates are already in points.
            1.0
        } else if self.scale.is_finite() && self.scale > 0.0 {
            self.scale
        } else {
            1.0
        }
    }

    /// The monitor's size in egui points.
    pub fn size_in_points(&self) -> (f32, f32) {
        let k = self.native_per_point();
        (self.width as f32 / k, self.height as f32 / k)
    }

    /// Where the top-left corner of a `panel_w`-point-wide panel goes so that
    /// it is centred on this monitor's top edge, `top_offset` points down, with
    /// `slide` points of extra vertical offset (negative = above the screen).
    pub fn panel_position(&self, panel_w: f32, top_offset: f32, slide: f32) -> (f32, f32) {
        let k = self.native_per_point();
        let (w, _) = self.size_in_points();
        let x = self.x as f32 / k + (w - panel_w) / 2.0;
        let y = self.y as f32 / k + top_offset + slide;
        (x, y)
    }

    /// The span of this monitor's top edge, in native x coordinates, where the
    /// pointer counts as touching the shelf's door: the middle part, as wide as
    /// the panel.
    pub fn edge_zone(&self, panel_w: f32) -> (i32, i32) {
        let k = self.native_per_point();
        let half = (panel_w * k / 2.0).round() as i64;
        let centre = self.x as i64 + self.width as i64 / 2;
        let left = (centre - half).max(self.x as i64);
        let right = (centre + half).min(self.x as i64 + self.width as i64);
        (left as i32, right as i32)
    }
}

/// All monitors, or an empty list when the platform will not say.
pub fn list() -> Vec<Monitor> {
    match display_info::DisplayInfo::all() {
        Ok(all) => all
            .into_iter()
            .map(|d| Monitor {
                x: d.x,
                y: d.y,
                width: d.width,
                height: d.height,
                scale: d.scale_factor,
                primary: d.is_primary,
            })
            .collect(),
        Err(e) => {
            log::debug!("could not list monitors: {e}");
            Vec::new()
        }
    }
}

pub fn at(monitors: &[Monitor], x: i32, y: i32) -> Option<Monitor> {
    monitors.iter().copied().find(|m| m.contains(x, y))
}

pub fn primary(monitors: &[Monitor]) -> Option<Monitor> {
    monitors
        .iter()
        .copied()
        .find(|m| m.primary)
        .or_else(|| monitors.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(x: i32, y: i32, w: u32, h: u32, scale: f32) -> Monitor {
        Monitor {
            x,
            y,
            width: w,
            height: h,
            scale,
            primary: false,
        }
    }

    #[test]
    fn containment_is_half_open() {
        let m = monitor(0, 0, 1920, 1080, 1.0);
        assert!(m.contains(0, 0));
        assert!(m.contains(1919, 1079));
        assert!(!m.contains(1920, 0));
        assert!(!m.contains(0, 1080));
        assert!(!m.contains(-1, 0));
    }

    #[test]
    fn monitors_to_the_left_have_negative_coordinates() {
        let m = monitor(-1920, 0, 1920, 1080, 1.0);
        assert!(m.contains(-1, 5));
        assert!(!m.contains(0, 5));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn panel_is_centred_in_points() {
        let m = monitor(0, 0, 2560, 1440, 2.0); // 1280x720 points
        let (x, y) = m.panel_position(800.0, 0.0, -380.0);
        assert_eq!(x, 240.0);
        assert_eq!(y, -380.0);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn panel_on_a_second_monitor_is_offset() {
        let m = monitor(1920, 0, 1920, 1080, 1.0);
        let (x, _) = m.panel_position(920.0, 0.0, 0.0);
        assert_eq!(x, 1920.0 + 500.0);
    }

    #[test]
    fn edge_zone_is_centred_and_clamped() {
        let m = monitor(0, 0, 1920, 1080, 1.0);
        assert_eq!(m.edge_zone(920.0), (500, 1420));
        // A panel wider than the screen covers all of it.
        assert_eq!(m.edge_zone(5000.0), (0, 1920));
    }

    #[test]
    fn unusable_scale_falls_back_to_one() {
        for scale in [0.0, -1.0, f32::NAN] {
            let m = monitor(0, 0, 100, 100, scale);
            #[cfg(not(target_os = "macos"))]
            assert_eq!(m.native_per_point(), 1.0);
            #[cfg(target_os = "macos")]
            let _ = m;
        }
    }

    #[test]
    fn lookup_helpers() {
        let a = Monitor {
            primary: true,
            ..monitor(0, 0, 100, 100, 1.0)
        };
        let b = monitor(100, 0, 100, 100, 1.0);
        let all = [b, a];
        assert_eq!(at(&all, 150, 5), Some(b));
        assert_eq!(at(&all, 500, 5), None);
        assert_eq!(primary(&all), Some(a));
        assert_eq!(primary(&[b]), Some(b));
        assert_eq!(primary(&[]), None);
    }
}
