//! The thread that watches the top edge of the screen.
//!
//! It polls the pointer about 20 times a second, but only while the edge
//! trigger is enabled and the shelf is hidden; otherwise it sleeps in long
//! steps just to notice a settings change. Polling is the only portable option:
//! no desktop lets an application subscribe to "the pointer touched the top".

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::cursor::PointerSource;
use super::edge::{EdgeConfig, EdgeDetector};
use super::monitors::{self, Monitor};
use crate::control::{Command, Control};

const POLL: Duration = Duration::from_millis(50);
const IDLE: Duration = Duration::from_millis(400);
/// Monitors rarely change; re-reading them is far more expensive than a pointer read.
const MONITOR_REFRESH: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WatchSettings {
    pub enabled: bool,
    pub dwell: Duration,
    pub panel_width: f32,
}

pub struct EdgeWatcher {
    settings: Arc<Mutex<WatchSettings>>,
    stop: Arc<AtomicBool>,
}

impl EdgeWatcher {
    /// Start watching. Returns `None` when this session cannot report the
    /// pointer (Wayland), in which case there is nothing to run.
    pub fn spawn(control: Control, initial: WatchSettings) -> Option<EdgeWatcher> {
        if !super::cursor::available() {
            return None;
        }
        let settings = Arc::new(Mutex::new(initial));
        let stop = Arc::new(AtomicBool::new(false));
        {
            let settings = settings.clone();
            let stop = stop.clone();
            std::thread::Builder::new()
                .name("tsumiba-edge".into())
                .spawn(move || run(control, settings, stop))
                .ok()?;
        }
        Some(EdgeWatcher { settings, stop })
    }

    pub fn update(&self, settings: WatchSettings) {
        *self.settings.lock().unwrap_or_else(|p| p.into_inner()) = settings;
    }
}

impl Drop for EdgeWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn run(control: Control, settings: Arc<Mutex<WatchSettings>>, stop: Arc<AtomicBool>) {
    let Some(mut source) = PointerSource::new() else {
        log::info!("the pointer cannot be read in this session; the edge trigger is off");
        return;
    };
    let mut detector = EdgeDetector::new();
    let mut monitors: Vec<Monitor> = Vec::new();
    let mut monitors_read: Option<Instant> = None;

    while !stop.load(Ordering::Relaxed) {
        let current = *settings.lock().unwrap_or_else(|p| p.into_inner());
        if !current.enabled {
            detector.suppress();
            std::thread::sleep(IDLE);
            continue;
        }
        if control.is_visible() {
            // Nothing to open. Keep the door shut until the pointer leaves it.
            detector.suppress();
            std::thread::sleep(POLL * 2);
            continue;
        }

        let now = Instant::now();
        if monitors_read.is_none_or(|t| now.duration_since(t) >= MONITOR_REFRESH) {
            monitors = monitors::list();
            monitors_read = Some(now);
        }

        let config = EdgeConfig {
            dwell: current.dwell,
            panel_width: current.panel_width,
        };
        if let Some(monitor) = detector.update(now, source.read(), &monitors, &config) {
            control.send(Command::ShowOn(monitor));
        }
        std::thread::sleep(POLL);
    }
}
