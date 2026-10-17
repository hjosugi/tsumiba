//! How the threads around the window talk to it.
//!
//! The hotkey, the tray icon, the pointer watcher and the HTTP server all run on
//! threads of their own. They tell the window what to do by sending a
//! [`Command`] through a [`Control`], which also wakes the event loop: a hidden
//! window sleeps until something asks it to repaint, so a command that did not
//! also wake the loop would sit in the queue unread.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, OnceLock};

use eframe::egui;

use crate::platform::monitors::Monitor;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Show,
    /// Show on a particular monitor (the one whose top edge was touched).
    ShowOn(Monitor),
    Hide,
    Toggle,
    ShowSettings,
    /// Something outside the window changed the shelf. Only wakes the loop.
    Wake,
    /// A message for the toast line.
    Notice(String),
    Quit,
}

/// A cloneable handle for sending commands to the window.
#[derive(Clone)]
pub struct Control {
    inner: Arc<Inner>,
}

struct Inner {
    tx: Sender<Command>,
    ctx: OnceLock<egui::Context>,
    visible: AtomicBool,
}

impl Control {
    pub fn new() -> (Control, Receiver<Command>) {
        let (tx, rx) = mpsc::channel();
        let control = Control {
            inner: Arc::new(Inner {
                tx,
                ctx: OnceLock::new(),
                visible: AtomicBool::new(false),
            }),
        };
        (control, rx)
    }

    /// Give the control the egui context so that sending also wakes the event
    /// loop. Until this is called (a headless `serve` never calls it) commands
    /// are queued and nobody is woken.
    pub fn attach(&self, ctx: egui::Context) {
        let _ = self.inner.ctx.set(ctx);
    }

    pub fn send(&self, command: Command) {
        // The receiver is gone only while the app is shutting down.
        let _ = self.inner.tx.send(command);
        self.wake();
    }

    pub fn wake(&self) {
        if let Some(ctx) = self.inner.ctx.get() {
            ctx.request_repaint();
        }
    }

    /// Whether the shelf is on screen. Written by the window, read by the
    /// pointer watcher and the API.
    pub fn is_visible(&self) -> bool {
        self.inner.visible.load(Ordering::Relaxed)
    }

    pub fn set_visible(&self, visible: bool) {
        self.inner.visible.store(visible, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_arrive_in_order() {
        let (control, rx) = Control::new();
        control.send(Command::Show);
        control.send(Command::Notice("hi".into()));
        control.send(Command::Quit);
        assert_eq!(rx.try_recv().unwrap(), Command::Show);
        assert_eq!(rx.try_recv().unwrap(), Command::Notice("hi".into()));
        assert_eq!(rx.try_recv().unwrap(), Command::Quit);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn visibility_is_shared_between_clones() {
        let (control, _rx) = Control::new();
        let other = control.clone();
        assert!(!other.is_visible());
        control.set_visible(true);
        assert!(other.is_visible());
    }

    #[test]
    fn sending_after_the_receiver_is_gone_does_not_panic() {
        let (control, rx) = Control::new();
        drop(rx);
        control.send(Command::Hide);
    }
}
