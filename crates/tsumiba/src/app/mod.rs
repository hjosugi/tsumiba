//! The window and everything in it.
//!
//! The window is created once, hidden, and then only ever shown and hidden: it
//! slides down from the top edge of a monitor when summoned and back up when it
//! is dismissed. While hidden it costs nothing — eframe sleeps until one of the
//! helper threads (hotkey, tray, pointer watcher, HTTP API) asks it to repaint,
//! and `App::logic`, which runs even for a hidden window, picks up their
//! [`Command`]s.

mod files;
mod fmt;
mod fonts;
mod notes;
mod panel;
mod settings;
mod slide;
pub mod theme;
mod widgets;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use eframe::egui::{self, ViewportCommand, vec2};
use tsumiba_core::config::StorageMode;
use tsumiba_core::runtime::ServerInfo;
use tsumiba_core::{Config, Deleted, Dirs, Store, token};

use crate::control::{Command, Control};
use crate::i18n::{self, Lang};
use crate::platform::hotkey::HotkeyService;
use crate::platform::monitors::{self, Monitor};
use crate::platform::tray::Tray;
use crate::platform::watcher::{EdgeWatcher, WatchSettings};
use crate::platform::{autostart, cursor, shell};
use crate::server::{ApiContext, Server, SharedConfig};

use self::notes::{NotesState, Selection};

const APP_ICON: &[u8] = include_bytes!("../../../../assets/icons/tsumiba-256.png");

/// How long the panel takes to slide in or out.
const SLIDE: Duration = Duration::from_millis(170);
/// Leave the shelf this long without the pointer before it hides itself.
const POINTER_GRACE: Duration = Duration::from_millis(700);
/// Save a note this long after the last keystroke.
const AUTOSAVE: Duration = Duration::from_millis(500);
const TOAST: Duration = Duration::from_secs(6);

/// macOS draws the menu bar over the top of every screen; start below it.
const TOP_INSET: f32 = if cfg!(target_os = "macos") { 32.0 } else { 0.0 };

/// Everything `main` prepares before the window exists.
pub struct Boot {
    pub dirs: Dirs,
    pub config: Config,
    /// A problem with `config.toml` to show once the window exists.
    pub config_error: Option<String>,
    pub store: Store,
    pub token: String,
    /// Open the panel right away (a manual launch) instead of waiting in the tray.
    pub show: bool,
}

/// What the views ask the app to do. Views only *describe* changes; the app
/// applies them after drawing, which keeps borrows simple and effects in one place.
#[derive(Debug)]
pub(crate) enum Action {
    NewNote,
    SelectNote(String),
    PinNote(String, bool),
    CopyText(String),
    DeleteNote(String),
    AddFilesDialog,
    OpenFile(String),
    RevealFile(String),
    CopyFile(String),
    CopyPath(String),
    SaveCopy(String),
    StartRename(String),
    PinFile(String, bool),
    RemoveFile(String),
    Hide,
    ToggleKeepOpen,
    OpenSettings,
    CloseSettings,
    Undo,
    DismissToast,
    ApplyHotkey,
    ApplyAutostart,
    CopyToken,
    RegenerateToken,
    OpenDataFolder,
    OpenConfigFile,
}

pub fn run(boot: Boot) -> eframe::Result {
    let wayland = cursor::is_wayland();
    let icon = image::load_from_memory(APP_ICON).ok().map(|img| {
        let rgba = img.into_rgba8();
        let (width, height) = rgba.dimensions();
        egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        }
    });

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("tsumiba")
        .with_app_id("tsumiba")
        .with_inner_size([boot.config.panel.width, boot.config.panel.height])
        .with_min_inner_size([480.0, 240.0])
        // A Wayland client cannot place or raise its own window, so there the
        // shelf is an ordinary window and keeps its title bar for moving/closing.
        .with_decorations(wayland)
        .with_resizable(wayland)
        .with_always_on_top()
        .with_taskbar(false)
        .with_visible(false)
        .with_window_type(egui::X11WindowType::Utility);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "tsumiba",
        options,
        Box::new(move |cc| Ok(Box::new(TsumibaApp::new(cc, boot)))),
    )
}

// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Hidden,
    Showing(slide::Slide),
    Shown,
    Hiding(slide::Slide),
}

/// Where the panel sits when fully shown, in egui points.
#[derive(Debug, Clone, Copy)]
struct Placement {
    x: f32,
    /// Top edge when fully shown.
    y: f32,
    height: f32,
}

struct WindowState {
    phase: Phase,
    placement: Option<Placement>,
    /// Current vertical offset from the resting position (0 = shown).
    offset: f32,
    /// The pointer has been over the panel, or it has had focus, since it appeared.
    engaged: bool,
    pointer_away_since: Option<Instant>,
    had_focus: bool,
}

impl WindowState {
    fn is_open(&self) -> bool {
        matches!(self.phase, Phase::Showing(_) | Phase::Shown)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Main,
    Settings,
}

struct Toast {
    text: String,
    undo: Option<Deleted>,
    until: Instant,
}

struct Caches {
    revision: u64,
    notes: Vec<tsumiba_core::Note>,
    files: Vec<tsumiba_core::StoredFile>,
}

pub struct TsumibaApp {
    dirs: Dirs,
    store: Store,
    /// The working copy of the configuration; `shared` is what other threads see.
    cfg: Config,
    shared: SharedConfig,
    control: Control,
    rx: Receiver<Command>,
    lang: Lang,
    token: String,

    server: Option<Server>,
    hotkey: HotkeyService,
    active_hotkey: String,
    watcher: Option<EdgeWatcher>,
    tray: Option<Tray>,

    window: WindowState,
    screen: Screen,
    keep_open: bool,
    quitting: bool,
    search: String,
    notes: NotesState,
    files: files::FilesState,
    settings: settings::SettingsState,
    caches: Caches,
    toast: Option<Toast>,
    /// Native dialogs currently open; the panel must not hide behind them.
    dialogs: Arc<AtomicUsize>,
}

impl TsumibaApp {
    fn new(cc: &eframe::CreationContext<'_>, boot: Boot) -> TsumibaApp {
        let ctx = cc.egui_ctx.clone();
        let (control, rx) = Control::new();
        control.attach(ctx.clone());

        theme::apply(&ctx, boot.config.general.theme);
        fonts::install_in_background(ctx.clone());
        egui_extras::install_image_loaders(&ctx);

        let lang = i18n::resolve(boot.config.general.language);
        let shared: SharedConfig = Arc::new(RwLock::new(boot.config.clone()));

        let mut toasts: Vec<String> = Vec::new();
        if let Some(error) = boot.config_error {
            toasts.push(error);
        }
        toasts.extend(boot.store.take_warnings());

        // The HTTP API for the command line and the browser extension.
        let mut server = None;
        if boot.config.api.enabled {
            let api = ApiContext {
                store: boot.store.clone(),
                control: control.clone(),
                token: boot.token.clone(),
                config: shared.clone(),
            };
            let wanted = boot.config.api.port;
            server = match Server::start(api.clone(), wanted) {
                Ok(server) => Some(server),
                Err(first) => {
                    log::warn!("could not listen on port {wanted}: {first}");
                    match Server::start(api, 0) {
                        Ok(server) => {
                            toasts.push(i18n::strings(lang).conflict_port(
                                lang,
                                wanted,
                                server.port(),
                            ));
                            Some(server)
                        }
                        Err(e) => {
                            log::error!("the API is unavailable: {e}");
                            None
                        }
                    }
                }
            };
            if let Some(server) = &server {
                let info = ServerInfo {
                    port: server.port(),
                    pid: std::process::id(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                };
                if let Err(e) = info.write(&boot.dirs.runtime_file()) {
                    log::warn!("could not record the running instance: {e}");
                }
            }
        }

        let mut hotkey = HotkeyService::new(control.clone());
        let mut active_hotkey = String::new();
        match hotkey.apply(&boot.config.trigger.hotkey) {
            Ok(()) => active_hotkey = boot.config.trigger.hotkey.clone(),
            Err(e) => {
                log::warn!("{e}");
                toasts.push(e);
            }
        }

        let watcher = EdgeWatcher::spawn(control.clone(), watch_settings(&boot.config));
        let tray = match Tray::new(control.clone(), i18n::strings(lang)) {
            Ok(tray) => Some(tray),
            Err(e) => {
                log::warn!("no tray icon: {e}");
                None
            }
        };

        let caches = Caches {
            revision: 0,
            notes: Vec::new(),
            files: Vec::new(),
        };

        let mut app = TsumibaApp {
            dirs: boot.dirs,
            store: boot.store,
            cfg: boot.config,
            shared,
            control: control.clone(),
            rx,
            lang,
            token: boot.token,
            server,
            hotkey,
            active_hotkey,
            watcher,
            tray,
            window: WindowState {
                phase: Phase::Hidden,
                placement: None,
                offset: 0.0,
                engaged: false,
                pointer_away_since: None,
                had_focus: false,
            },
            screen: Screen::Main,
            keep_open: false,
            quitting: false,
            search: String::new(),
            notes: NotesState::default(),
            files: files::FilesState::default(),
            settings: settings::SettingsState::default(),
            caches,
            toast: None,
            dialogs: Arc::new(AtomicUsize::new(0)),
        };
        if let Some(first) = toasts.into_iter().next() {
            app.set_toast(first, None);
        }
        if boot.show {
            control.send(Command::Show);
        }
        app
    }

    // ---- commands ---------------------------------------------------------

    fn drain_commands(&mut self, ctx: &egui::Context) {
        let commands: Vec<Command> = self.rx.try_iter().collect();
        for command in commands {
            match command {
                Command::Show => self.show(ctx, None),
                Command::ShowOn(monitor) => self.show(ctx, Some(monitor)),
                Command::Hide => self.hide(ctx),
                Command::Toggle => {
                    if self.window.is_open() {
                        self.hide(ctx);
                    } else {
                        self.show(ctx, None);
                    }
                }
                Command::ShowSettings => {
                    self.screen = Screen::Settings;
                    self.show(ctx, None);
                }
                Command::Wake => {}
                Command::Notice(text) => self.set_toast(text, None),
                Command::Quit => self.quit(ctx),
            }
        }
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.flush_note();
        self.quitting = true;
        ctx.send_viewport_cmd(ViewportCommand::Close);
    }

    // ---- showing and hiding ----------------------------------------------

    fn show(&mut self, ctx: &egui::Context, monitor: Option<Monitor>) {
        if self.window.is_open() {
            ctx.send_viewport_cmd(ViewportCommand::Focus);
            return;
        }

        // Pick up anything that changed on disk while we were away, and start
        // with an empty note ready for typing.
        let _ = self.store.refresh();
        self.reconcile_caches();
        if self.screen == Screen::Main {
            self.notes.start_draft();
        }
        self.window.engaged = false;
        self.window.pointer_away_since = None;
        self.window.had_focus = false;

        let monitors = monitors::list();
        let target = monitor
            .or_else(|| {
                cursor::PointerSource::new()
                    .and_then(|mut p| p.read())
                    .and_then(|p| monitors::at(&monitors, p.x, p.y))
            })
            .or_else(|| monitors::primary(&monitors));

        let (screen_w, screen_h) = target
            .map(|m| m.size_in_points())
            .unwrap_or((1920.0, 1080.0));
        let width = self.cfg.panel.width.min(screen_w - 16.0).max(480.0);
        let height = self.cfg.panel.height.min(screen_h - 80.0).max(240.0);
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(width, height)));

        let animate = !cursor::is_wayland() && target.is_some();
        if let (true, Some(m)) = (animate, target) {
            let (x, y) = m.panel_position(width, TOP_INSET, 0.0);
            self.window.placement = Some(Placement { x, y, height });
            self.window.offset = -height;
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(x, y - height)));
            self.window.phase =
                Phase::Showing(slide::Slide::new(-height, 0.0, Instant::now(), SLIDE));
        } else {
            self.window.placement = None;
            self.window.offset = 0.0;
            self.window.phase = Phase::Shown;
        }

        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        self.control.set_visible(true);
        ctx.request_repaint();
    }

    fn hide(&mut self, ctx: &egui::Context) {
        if !self.window.is_open() {
            return;
        }
        self.flush_note();
        self.keep_open = false;
        self.files.rename = None;
        self.settings.capturing_hotkey = false;
        self.screen = Screen::Main;

        match self.window.placement {
            Some(p) => {
                let slide = slide::Slide::new(self.window.offset, -p.height, Instant::now(), SLIDE);
                self.window.phase = Phase::Hiding(slide);
                ctx.request_repaint();
            }
            None => self.finish_hide(ctx),
        }
    }

    fn finish_hide(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        self.window.phase = Phase::Hidden;
        self.control.set_visible(false);
    }

    fn move_to(&self, ctx: &egui::Context, offset: f32) {
        if let Some(p) = self.window.placement {
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(egui::pos2(
                p.x,
                p.y + offset,
            )));
        }
    }

    fn tick_animation(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        match self.window.phase {
            Phase::Showing(slide) => {
                let (offset, done) = slide.at(now);
                self.window.offset = offset;
                self.move_to(ctx, offset);
                if done {
                    self.window.phase = Phase::Shown;
                } else {
                    ctx.request_repaint();
                }
            }
            Phase::Hiding(slide) => {
                let (offset, done) = slide.at(now);
                self.window.offset = offset;
                self.move_to(ctx, offset);
                if done {
                    self.finish_hide(ctx);
                } else {
                    ctx.request_repaint();
                }
            }
            Phase::Hidden | Phase::Shown => {}
        }
    }

    /// Closing the window (the title bar's ✕ on Wayland, Alt+F4) hides the
    /// shelf; quitting is the tray menu's job.
    fn tick_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.hide(ctx);
        }
    }

    fn tick_auto_hide(&mut self, ctx: &egui::Context) {
        if self.window.phase != Phase::Shown {
            return;
        }
        let (inside, focused, dropping) = ctx.input(|i| {
            (
                i.pointer.has_pointer(),
                i.viewport().focused,
                !i.raw.hovered_files.is_empty(),
            )
        });
        if inside || focused == Some(true) {
            self.window.engaged = true;
        }
        if focused == Some(true) {
            self.window.had_focus = true;
        }

        let editing = self.notes.has_unsaved_text() && ctx.memory(|m| m.focused().is_some());
        let busy = self.keep_open
            || !self.cfg.panel.auto_hide
            || editing
            || dropping
            || self.files.rename.is_some()
            || self.settings.capturing_hotkey
            || self.dialogs.load(Ordering::Relaxed) > 0
            || egui::Popup::is_any_open(ctx)
            || self.screen == Screen::Settings;
        if busy {
            self.window.pointer_away_since = None;
            return;
        }

        // Clicking another window takes focus away: that means "I'm done".
        if self.window.had_focus && focused == Some(false) {
            self.hide(ctx);
            return;
        }
        if inside || !self.window.engaged {
            self.window.pointer_away_since = None;
            return;
        }
        let since = *self
            .window
            .pointer_away_since
            .get_or_insert_with(Instant::now);
        if since.elapsed() >= POINTER_GRACE {
            self.hide(ctx);
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    // ---- the store --------------------------------------------------------

    /// Refresh the cached lists when the store changed, and keep the editor
    /// consistent with what is stored.
    fn reconcile_caches(&mut self) {
        let revision = self.store.revision();
        if revision == self.caches.revision {
            return;
        }
        self.caches.revision = revision;
        self.caches.notes = self.store.notes();
        self.caches.files = self.store.files();

        if let Selection::Note(id) = self.notes.selected.clone() {
            match self.caches.notes.iter().find(|n| n.id == id) {
                Some(note) => {
                    // Changed elsewhere (the API, a sync tool) and we have no
                    // unsaved edits: show the new text.
                    if self.notes.dirty_since.is_none() && note.body != self.notes.loaded {
                        self.notes.buffer = note.body.clone();
                        self.notes.loaded = note.body.clone();
                    }
                }
                None => self.notes.clear(),
            }
        }
        if let Some(id) = &self.files.selected
            && !self.caches.files.iter().any(|f| &f.id == id)
        {
            self.files.selected = None;
        }
    }

    /// Write the editor's text to the store if it has changed.
    fn flush_note(&mut self) {
        match self.notes.selected.clone() {
            Selection::None => {}
            Selection::Draft => {
                if !self.notes.buffer.trim().is_empty() {
                    match self.store.add_note(&self.notes.buffer) {
                        Ok(note) => {
                            self.notes.selected = Selection::Note(note.id);
                            self.notes.loaded = self.notes.buffer.clone();
                            self.notes.dirty_since = None;
                        }
                        Err(e) => self.set_toast(e.to_string(), None),
                    }
                }
            }
            Selection::Note(id) => {
                if self.notes.buffer == self.notes.loaded {
                    self.notes.dirty_since = None;
                } else if self.notes.buffer.trim().is_empty() {
                    // Emptying a note deletes it (it can be undone).
                    self.delete_note(&id);
                } else {
                    match self.store.update_note(&id, &self.notes.buffer) {
                        Ok(_) => {
                            self.notes.loaded = self.notes.buffer.clone();
                            self.notes.dirty_since = None;
                        }
                        Err(e) => self.set_toast(e.to_string(), None),
                    }
                }
            }
        }
        self.reconcile_caches();
    }

    fn delete_note(&mut self, id: &str) {
        if self.notes.selected_id() == Some(id) {
            self.notes.clear();
        }
        match self.store.delete_note(id) {
            Ok(deleted) => {
                let text = i18n::strings(self.lang).note_deleted.to_string();
                self.set_toast(text, Some(deleted));
            }
            Err(e) => self.set_toast(e.to_string(), None),
        }
    }

    fn set_toast(&mut self, text: String, undo: Option<Deleted>) {
        self.toast = Some(Toast {
            text,
            undo,
            until: Instant::now() + TOAST,
        });
    }

    // ---- files ------------------------------------------------------------

    fn import_paths(&self, paths: Vec<PathBuf>) {
        import_in_background(
            self.store.clone(),
            self.cfg.storage.mode,
            self.control.clone(),
            self.lang,
            paths,
        );
    }

    fn take_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = dropped.into_iter().filter_map(|f| f.path).collect();
        if !paths.is_empty() {
            self.import_paths(paths);
        }
    }

    fn open_dialog_add_files(&self) {
        let dialogs = self.dialogs.clone();
        let store = self.store.clone();
        let mode = self.cfg.storage.mode;
        let control = self.control.clone();
        let lang = self.lang;
        dialogs.fetch_add(1, Ordering::Relaxed);
        let spawned = std::thread::Builder::new()
            .name("tsumiba-dialog".into())
            .spawn(move || {
                let picked = pollster::block_on(rfd::AsyncFileDialog::new().pick_files());
                dialogs.fetch_sub(1, Ordering::Relaxed);
                if let Some(files) = picked {
                    let paths = files.into_iter().map(|f| f.path().to_path_buf()).collect();
                    import_in_background(store, mode, control, lang, paths);
                }
            });
        if spawned.is_err() {
            self.dialogs.fetch_sub(1, Ordering::Relaxed);
        }
    }

    fn open_dialog_save_copy(&self, id: &str) {
        let Some(file) = self.store.file(id) else {
            return;
        };
        let dialogs = self.dialogs.clone();
        let control = self.control.clone();
        let lang = self.lang;
        dialogs.fetch_add(1, Ordering::Relaxed);
        let spawned = std::thread::Builder::new()
            .name("tsumiba-dialog".into())
            .spawn(move || {
                let target = pollster::block_on(
                    rfd::AsyncFileDialog::new()
                        .set_file_name(&file.name)
                        .save_file(),
                );
                dialogs.fetch_sub(1, Ordering::Relaxed);
                if let Some(target) = target {
                    let s = i18n::strings(lang);
                    let message = match std::fs::copy(&file.path, target.path()) {
                        Ok(_) => s.copied.to_string(),
                        Err(e) => s.add_failed(lang, &file.name, &e.to_string()),
                    };
                    control.send(Command::Notice(message));
                }
            });
        if spawned.is_err() {
            self.dialogs.fetch_sub(1, Ordering::Relaxed);
        }
    }

    // ---- settings ---------------------------------------------------------

    /// Persist and apply the working configuration after the settings screen changed it.
    fn config_changed(&mut self, ctx: &egui::Context) {
        self.cfg = std::mem::take(&mut self.cfg).sanitized();
        if let Err(e) = self.cfg.save(&self.dirs.config_file()) {
            self.set_toast(e.to_string(), None);
        }
        if let Ok(mut shared) = self.shared.write() {
            *shared = self.cfg.clone();
        }
        theme::apply(ctx, self.cfg.general.theme);
        let lang = i18n::resolve(self.cfg.general.language);
        if lang != self.lang {
            self.lang = lang;
            if let Some(tray) = &self.tray {
                tray.set_language(i18n::strings(lang));
            }
        }
        if let Some(watcher) = &self.watcher {
            watcher.update(watch_settings(&self.cfg));
        }
        // Resize live so the sliders give feedback.
        if self.window.is_open() {
            self.window.placement = None;
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(
                self.cfg.panel.width,
                self.cfg.panel.height,
            )));
        }
    }

    fn apply_hotkey(&mut self) {
        match self.hotkey.apply(&self.cfg.trigger.hotkey) {
            Ok(()) => {
                self.active_hotkey = self.cfg.trigger.hotkey.clone();
                self.settings.hotkey_error = None;
            }
            Err(e) => {
                // Keep what works.
                self.cfg.trigger.hotkey = self.active_hotkey.clone();
                self.settings.hotkey_error = Some(e);
            }
        }
    }

    fn apply_autostart(&mut self) {
        if let Err(e) = autostart::set_enabled(self.cfg.general.launch_at_login) {
            self.cfg.general.launch_at_login = autostart::is_enabled();
            self.set_toast(e, None);
        }
    }

    // ---- actions ----------------------------------------------------------

    fn apply(&mut self, actions: Vec<Action>, ctx: &egui::Context) {
        let s = i18n::strings(self.lang);
        for action in actions {
            match action {
                Action::NewNote => {
                    self.flush_note();
                    self.notes.start_draft();
                }
                Action::SelectNote(id) => {
                    self.flush_note();
                    if let Some(note) = self.caches.notes.iter().find(|n| n.id == id).cloned() {
                        self.notes.open(&note);
                    }
                }
                Action::PinNote(id, pinned) => {
                    if let Err(e) = self.store.set_note_pinned(&id, pinned) {
                        self.set_toast(e.to_string(), None);
                    }
                }
                Action::CopyText(text) => {
                    ctx.copy_text(text);
                    self.set_toast(s.copied.to_string(), None);
                }
                Action::DeleteNote(id) => self.delete_note(&id),
                Action::AddFilesDialog => self.open_dialog_add_files(),
                Action::OpenFile(id) => {
                    if let Some(file) = self.store.file(&id)
                        && let Err(e) = shell::open(&file.path)
                    {
                        self.set_toast(s.add_failed(self.lang, &file.name, &e.to_string()), None);
                    }
                }
                Action::RevealFile(id) => {
                    if let Some(file) = self.store.file(&id)
                        && let Err(e) = shell::reveal(&file.path)
                    {
                        self.set_toast(s.add_failed(self.lang, &file.name, &e.to_string()), None);
                    }
                }
                Action::CopyFile(id) => {
                    if let Some(file) = self.store.file(&id) {
                        match shell::copy_file_to_clipboard(&file.path) {
                            Ok(()) => self.set_toast(s.copied.to_string(), None),
                            Err(e) => self.set_toast(s.add_failed(self.lang, &file.name, &e), None),
                        }
                    }
                }
                Action::CopyPath(id) => {
                    if let Some(file) = self.store.file(&id) {
                        ctx.copy_text(file.path.to_string_lossy().into_owned());
                        self.set_toast(s.copied.to_string(), None);
                    }
                }
                Action::SaveCopy(id) => self.open_dialog_save_copy(&id),
                Action::StartRename(id) => {
                    if let Some(file) = self.store.file(&id) {
                        self.files.rename = Some(files::RenameState {
                            id,
                            text: file.name,
                            focus: true,
                        });
                    }
                }
                Action::PinFile(id, pinned) => {
                    if let Err(e) = self.store.set_file_pinned(&id, pinned) {
                        self.set_toast(e.to_string(), None);
                    }
                }
                Action::RemoveFile(id) => {
                    if let Some(file) = self.store.file(&id)
                        && let Some(uri) = files::thumbnail_uri(&file)
                    {
                        ctx.forget_image(&uri);
                    }
                    match self.store.delete_file(&id) {
                        Ok(deleted) => self.set_toast(s.file_removed.to_string(), Some(deleted)),
                        Err(e) => self.set_toast(e.to_string(), None),
                    }
                }
                Action::Hide => self.hide(ctx),
                Action::ToggleKeepOpen => self.keep_open = !self.keep_open,
                Action::OpenSettings => {
                    self.flush_note();
                    self.screen = Screen::Settings;
                }
                Action::CloseSettings => {
                    self.screen = Screen::Main;
                    self.settings.capturing_hotkey = false;
                }
                Action::Undo => {
                    if let Some(deleted) = self.toast.take().and_then(|t| t.undo)
                        && let Err(e) = self.store.restore(&deleted)
                    {
                        self.set_toast(e.to_string(), None);
                    }
                }
                Action::DismissToast => self.toast = None,
                Action::ApplyHotkey => self.apply_hotkey(),
                Action::ApplyAutostart => self.apply_autostart(),
                Action::CopyToken => {
                    ctx.copy_text(self.token.clone());
                    self.set_toast(s.copied.to_string(), None);
                }
                Action::RegenerateToken => match token::regenerate(&self.dirs.token_file()) {
                    Ok(new_token) => {
                        // The server compares against its own copy, so restart it
                        // with the new secret.
                        self.token = new_token;
                        self.restart_server();
                    }
                    Err(e) => self.set_toast(e.to_string(), None),
                },
                Action::OpenDataFolder => {
                    let _ = shell::open(self.store.root());
                }
                Action::OpenConfigFile => {
                    if !self.dirs.config_file().exists() {
                        let _ = self.cfg.save(&self.dirs.config_file());
                    }
                    let _ = shell::open(&self.dirs.config_file());
                }
            }
        }
    }

    fn restart_server(&mut self) {
        if let Some(server) = self.server.take() {
            server.shutdown();
        }
        if !self.cfg.api.enabled {
            return;
        }
        let api = ApiContext {
            store: self.store.clone(),
            control: self.control.clone(),
            token: self.token.clone(),
            config: self.shared.clone(),
        };
        self.server = Server::start(api.clone(), self.cfg.api.port)
            .or_else(|_| Server::start(api, 0))
            .ok();
        if let Some(server) = &self.server {
            let info = ServerInfo {
                port: server.port(),
                pid: std::process::id(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            };
            let _ = info.write(&self.dirs.runtime_file());
        }
    }
}

fn watch_settings(config: &Config) -> WatchSettings {
    WatchSettings {
        enabled: config.trigger.edge,
        dwell: Duration::from_millis(u64::from(config.trigger.edge_dwell_ms)),
        panel_width: config.panel.width,
    }
}

/// Copy files into the shelf on a worker thread and report through a toast.
fn import_in_background(
    store: Store,
    mode: StorageMode,
    control: Control,
    lang: Lang,
    paths: Vec<PathBuf>,
) {
    let spawned = std::thread::Builder::new()
        .name("tsumiba-import".into())
        .spawn(move || {
            let s = i18n::strings(lang);
            let mut added = 0;
            let mut first_error: Option<String> = None;
            for path in &paths {
                match store.add_path(path, mode) {
                    Ok(_) => added += 1,
                    Err(e) => {
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string());
                        first_error
                            .get_or_insert_with(|| s.add_failed(lang, &name, &e.to_string()));
                    }
                }
            }
            let message = match (added, first_error) {
                (0, Some(error)) => error,
                (n, Some(error)) => format!("{} — {error}", s.files_added(lang, n)),
                (n, None) => s.files_added(lang, n),
            };
            control.send(Command::Notice(message));
        });
    if let Err(e) = spawned {
        log::error!("could not start the importer: {e}");
    }
}

impl eframe::App for TsumibaApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_commands(ctx);
        self.tick_close_request(ctx);
        self.tick_animation(ctx);
        self.tick_auto_hide(ctx);

        if self.window.is_open() {
            if let Some(since) = self.notes.dirty_since {
                if since.elapsed() >= AUTOSAVE {
                    self.flush_note();
                } else {
                    ctx.request_repaint_after(AUTOSAVE - since.elapsed());
                }
            }
            if let Some(toast) = &self.toast {
                if Instant::now() >= toast.until {
                    self.toast = None;
                } else {
                    ctx.request_repaint_after(toast.until - Instant::now());
                }
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        panel::draw(self, ui);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        let colour = if visuals.dark_mode {
            theme::palette(true).bg
        } else {
            theme::palette(false).bg
        };
        egui::Rgba::from(colour).to_array()
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.flush_note();
        if let Some(server) = self.server.take() {
            server.shutdown();
        }
        let _ = ServerInfo::remove(&self.dirs.runtime_file());
    }
}
