//! The "door" at the top of the screen.
//!
//! [`EdgeDetector`] is a pure state machine: it is fed pointer readings and says
//! when the shelf should open. Keeping it free of threads and clocks of its own
//! makes the timing rules testable.
//!
//! The rules:
//!
//! - the pointer must touch the top row of a monitor, in the middle part of it
//!   (as wide as the panel, so the panel appears where you are pointing);
//! - it must rest there for the configured delay, so brushing past the top of
//!   the screen to reach a window's title bar does not open anything;
//! - with the primary button held (you are dragging something) there is no
//!   delay: dragging a file to the top opens the shelf at once;
//! - after opening, nothing more fires until the pointer has left the zone.

use std::time::{Duration, Instant};

use super::cursor::Pointer;
use super::monitors::{self, Monitor};

#[derive(Debug, Clone, Copy)]
pub struct EdgeConfig {
    pub dwell: Duration,
    /// Width of the panel in egui points; sets the width of the active zone.
    pub panel_width: f32,
}

#[derive(Debug, Default)]
pub struct EdgeDetector {
    resting: Option<Resting>,
    fired: bool,
}

#[derive(Debug, Clone, Copy)]
struct Resting {
    since: Instant,
    monitor: (i32, i32),
}

/// How close to the top of a monitor, in native units, still counts as touching
/// it. Pointers on scaled displays often stop one pixel short of the edge.
const TOP_ROW: i32 = 1;

impl EdgeDetector {
    pub fn new() -> EdgeDetector {
        EdgeDetector::default()
    }

    /// Feed one reading. Returns the monitor to open the shelf on, once.
    pub fn update(
        &mut self,
        now: Instant,
        pointer: Option<Pointer>,
        monitors: &[Monitor],
        config: &EdgeConfig,
    ) -> Option<Monitor> {
        let Some(pointer) = pointer else {
            self.leave();
            return None;
        };
        let Some(monitor) = monitors::at(monitors, pointer.x, pointer.y) else {
            self.leave();
            return None;
        };

        let (left, right) = monitor.edge_zone(config.panel_width);
        let in_zone = pointer.y - monitor.y <= TOP_ROW && pointer.x >= left && pointer.x < right;
        if !in_zone {
            self.leave();
            return None;
        }
        if self.fired {
            return None;
        }

        let key = (monitor.x, monitor.y);
        let resting = match self.resting {
            Some(r) if r.monitor == key => r,
            _ => {
                let r = Resting {
                    since: now,
                    monitor: key,
                };
                self.resting = Some(r);
                r
            }
        };

        let needed = if pointer.primary_down {
            Duration::ZERO
        } else {
            config.dwell
        };
        if now.saturating_duration_since(resting.since) >= needed {
            self.fired = true;
            return Some(monitor);
        }
        None
    }

    /// The shelf is open (or was just closed by something else). Do not fire
    /// again until the pointer has gone away and come back.
    pub fn suppress(&mut self) {
        self.fired = true;
        self.resting = None;
    }

    fn leave(&mut self) {
        self.resting = None;
        self.fired = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DWELL: Duration = Duration::from_millis(250);

    fn monitor(x: i32, w: u32) -> Monitor {
        Monitor {
            x,
            y: 0,
            width: w,
            height: 1080,
            scale: 1.0,
            primary: x == 0,
        }
    }

    fn config() -> EdgeConfig {
        EdgeConfig {
            dwell: DWELL,
            panel_width: 920.0,
        }
    }

    fn at(x: i32, y: i32, down: bool) -> Option<Pointer> {
        Some(Pointer {
            x,
            y,
            primary_down: down,
        })
    }

    fn ms(base: Instant, n: u64) -> Instant {
        base + Duration::from_millis(n)
    }

    #[test]
    fn rests_long_enough_then_fires_once() {
        let m = [monitor(0, 1920)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        assert_eq!(d.update(t0, at(960, 0, false), &m, &config()), None);
        assert_eq!(
            d.update(ms(t0, 100), at(960, 0, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 249), at(960, 0, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 250), at(960, 0, false), &m, &config()),
            Some(m[0])
        );
        // Still resting: nothing more.
        assert_eq!(
            d.update(ms(t0, 400), at(960, 0, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 4000), at(960, 0, false), &m, &config()),
            None
        );
    }

    #[test]
    fn brushing_past_does_not_fire() {
        let m = [monitor(0, 1920)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        assert_eq!(d.update(t0, at(960, 0, false), &m, &config()), None);
        // Leaves the edge before the delay is up, then comes back: the clock restarts.
        assert_eq!(
            d.update(ms(t0, 100), at(960, 40, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 200), at(960, 0, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 400), at(960, 0, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 450), at(960, 0, false), &m, &config()),
            Some(m[0])
        );
    }

    #[test]
    fn dragging_opens_immediately() {
        let m = [monitor(0, 1920)];
        let mut d = EdgeDetector::new();
        assert_eq!(
            d.update(Instant::now(), at(960, 0, true), &m, &config()),
            Some(m[0])
        );
    }

    #[test]
    fn only_the_middle_of_the_edge_counts() {
        let m = [monitor(0, 1920)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        // Zone is x in [500, 1420).
        for x in [0, 499, 1420, 1919] {
            assert_eq!(d.update(t0, at(x, 0, true), &m, &config()), None, "x = {x}");
        }
        assert_eq!(d.update(t0, at(500, 0, true), &m, &config()), Some(m[0]));
    }

    #[test]
    fn one_pixel_short_of_the_edge_still_counts() {
        let m = [monitor(0, 1920)];
        let mut d = EdgeDetector::new();
        assert_eq!(
            d.update(Instant::now(), at(960, 1, true), &m, &config()),
            Some(m[0])
        );
        let mut d = EdgeDetector::new();
        assert_eq!(
            d.update(Instant::now(), at(960, 2, true), &m, &config()),
            None
        );
    }

    #[test]
    fn leaving_the_zone_rearms_it() {
        let m = [monitor(0, 1920)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        assert!(d.update(t0, at(960, 0, true), &m, &config()).is_some());
        assert_eq!(d.update(ms(t0, 50), at(960, 0, true), &m, &config()), None);
        assert_eq!(
            d.update(ms(t0, 100), at(960, 500, true), &m, &config()),
            None
        );
        assert!(
            d.update(ms(t0, 150), at(960, 0, true), &m, &config())
                .is_some()
        );
    }

    #[test]
    fn suppress_holds_fire_until_the_pointer_leaves() {
        let m = [monitor(0, 1920)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        d.suppress();
        assert_eq!(d.update(t0, at(960, 0, true), &m, &config()), None);
        assert_eq!(
            d.update(ms(t0, 10), at(960, 300, true), &m, &config()),
            None
        );
        assert!(
            d.update(ms(t0, 20), at(960, 0, true), &m, &config())
                .is_some()
        );
    }

    #[test]
    fn each_monitor_has_its_own_door() {
        let m = [monitor(0, 1920), monitor(1920, 2560)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        // Zone on the second monitor: centre 1920 + 1280 = 3200, ±460.
        assert_eq!(d.update(t0, at(3200, 0, true), &m, &config()), Some(m[1]));
    }

    #[test]
    fn moving_between_monitors_restarts_the_clock() {
        let m = [monitor(0, 1920), monitor(1920, 1920)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        assert_eq!(d.update(t0, at(960, 0, false), &m, &config()), None);
        // Jumps to the other monitor's door at 200 ms: the 250 ms must be served there.
        assert_eq!(
            d.update(ms(t0, 200), at(2880, 0, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 400), at(2880, 0, false), &m, &config()),
            None
        );
        assert_eq!(
            d.update(ms(t0, 450), at(2880, 0, false), &m, &config()),
            Some(m[1])
        );
    }

    #[test]
    fn unreadable_pointer_resets() {
        let m = [monitor(0, 1920)];
        let t0 = Instant::now();
        let mut d = EdgeDetector::new();
        assert_eq!(d.update(t0, at(960, 0, false), &m, &config()), None);
        assert_eq!(d.update(ms(t0, 100), None, &m, &config()), None);
        assert_eq!(
            d.update(ms(t0, 300), at(960, 0, false), &m, &config()),
            None
        );
    }

    #[test]
    fn zero_delay_fires_at_once() {
        let m = [monitor(0, 1920)];
        let mut d = EdgeDetector::new();
        let cfg = EdgeConfig {
            dwell: Duration::ZERO,
            panel_width: 920.0,
        };
        assert!(
            d.update(Instant::now(), at(960, 0, false), &m, &cfg)
                .is_some()
        );
    }
}
