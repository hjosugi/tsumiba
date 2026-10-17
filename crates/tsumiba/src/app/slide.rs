//! The motion of the panel sliding in from (and back out of) the screen edge.

use std::time::{Duration, Instant};

/// A one-dimensional move from `from` to `to`, eased out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slide {
    from: f32,
    to: f32,
    start: Instant,
    duration: Duration,
}

impl Slide {
    pub fn new(from: f32, to: f32, start: Instant, duration: Duration) -> Slide {
        Slide {
            from,
            to,
            start,
            duration,
        }
    }

    /// The position at `now` and whether the move is over.
    pub fn at(&self, now: Instant) -> (f32, bool) {
        if self.duration.is_zero() {
            return (self.to, true);
        }
        let elapsed = now.saturating_duration_since(self.start).as_secs_f32();
        let t = (elapsed / self.duration.as_secs_f32()).clamp(0.0, 1.0);
        (
            self.from + (self.to - self.from) * ease_out_cubic(t),
            t >= 1.0,
        )
    }
}

/// Fast at first, settling gently: the way a panel feels when it drops into place.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_is_pinned_at_both_ends_and_monotonic() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert_eq!(ease_out_cubic(1.0), 1.0);
        let mut last = 0.0;
        for i in 1..=100 {
            let v = ease_out_cubic(i as f32 / 100.0);
            assert!(v >= last, "not monotonic at step {i}");
            last = v;
        }
        // Out-of-range input is clamped.
        assert_eq!(ease_out_cubic(-1.0), 0.0);
        assert_eq!(ease_out_cubic(2.0), 1.0);
    }

    #[test]
    fn easing_front_loads_the_motion() {
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn slide_runs_from_start_to_end() {
        let t0 = Instant::now();
        let slide = Slide::new(-400.0, 0.0, t0, Duration::from_millis(200));
        assert_eq!(slide.at(t0), (-400.0, false));
        let (mid, done) = slide.at(t0 + Duration::from_millis(100));
        assert!(mid > -200.0 && mid < 0.0 && !done, "{mid}");
        assert_eq!(slide.at(t0 + Duration::from_millis(200)), (0.0, true));
        assert_eq!(slide.at(t0 + Duration::from_secs(5)), (0.0, true));
    }

    #[test]
    fn slide_can_run_backwards() {
        let t0 = Instant::now();
        let slide = Slide::new(0.0, -400.0, t0, Duration::from_millis(100));
        assert_eq!(slide.at(t0 + Duration::from_millis(100)), (-400.0, true));
    }

    #[test]
    fn zero_duration_finishes_immediately() {
        let t0 = Instant::now();
        assert_eq!(Slide::new(5.0, 9.0, t0, Duration::ZERO).at(t0), (9.0, true));
    }

    #[test]
    fn a_clock_that_runs_backwards_does_not_overshoot() {
        let t0 = Instant::now() + Duration::from_secs(1);
        let slide = Slide::new(0.0, 10.0, t0, Duration::from_millis(100));
        assert_eq!(slide.at(t0 - Duration::from_millis(500)), (0.0, false));
    }
}
