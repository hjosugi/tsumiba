//! The shelf: notes and files kept in one directory.
//!
//! ```text
//! <data>/
//!   index.json                 pinned notes + metadata of the stored files
//!   notes/<id>.md              one Markdown file per note
//!   files/<id>/<name>          one directory per stored file
//!   trash/<deleted_ms>_<id>/   deleted items, purged after 30 days
//! ```
//!
//! Everything that can be derived from the disk is derived from the disk: a
//! note's creation time is in its id, its update time is the file's mtime, and
//! a file or note that exists on disk but is missing from the index is adopted
//! the next time the directory is scanned. That makes the layout safe to sync
//! with a file-synchronisation tool and lets a damaged `index.json` be rebuilt.
//!
//! [`Store`] is a cheap handle that can be cloned across threads. Copying a big
//! file happens *outside* the internal lock (see [`Store::stage_copy`]), so the
//! UI never waits on a slow disk.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::StorageMode;
use crate::error::{Error, IoContext, Result};
use crate::fsutil::{copy_new, sanitize_file_name, write_atomic};
use crate::id;

const INDEX_VERSION: u32 = 1;
/// A note body larger than this is almost certainly not a note.
pub const MAX_NOTE_BYTES: usize = 4 * 1024 * 1024;
/// How long deleted items stay recoverable.
pub const TRASH_RETENTION: Duration = Duration::from_secs(30 * 24 * 60 * 60);

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub body: String,
    pub pinned: bool,
    pub created_ms: u64,
    pub updated_ms: u64,
}

impl Note {
    /// The first non-empty line, without Markdown heading markers. May be empty.
    pub fn title(&self) -> String {
        let line = self.body.lines().map(str::trim).find(|l| !l.is_empty());
        let line = line.unwrap_or("");
        let line = line.trim_start_matches('#').trim();
        truncate_chars(line, 80)
    }

    /// The text after the title, collapsed onto one line, for list previews.
    pub fn snippet(&self) -> String {
        let mut lines = self.body.lines().map(str::trim).filter(|l| !l.is_empty());
        lines.next(); // the title
        let rest: Vec<&str> = lines.collect();
        truncate_chars(&rest.join(" "), 140)
    }

    /// Case-insensitive substring match on the body. An empty query matches.
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim();
        q.is_empty() || self.body.to_lowercase().contains(&q.to_lowercase())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredFile {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub added_ms: u64,
    pub pinned: bool,
    pub source: Option<String>,
    /// Where the file really is. For a copy this is inside the data directory,
    /// for a link it is the original location.
    pub path: PathBuf,
    /// `true` when only the location is remembered.
    pub link: bool,
    /// `true` when the item is a folder (always a link).
    pub is_dir: bool,
    /// `true` when the file is no longer where it should be.
    pub missing: bool,
}

impl StoredFile {
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim().to_lowercase();
        q.is_empty()
            || self.name.to_lowercase().contains(&q)
            || self
                .source
                .as_deref()
                .is_some_and(|s| s.to_lowercase().contains(&q))
    }

    /// Lowercase extension without the dot, or an empty string.
    pub fn extension(&self) -> String {
        Path::new(&self.name)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }
}

/// A file that has been written to the shelf's directory but not yet added to
/// the index. Produced by the `stage_*` methods, consumed by [`Store::commit`].
#[derive(Debug)]
pub struct Staged {
    id: String,
    name: String,
    size: u64,
    link: Option<String>,
    is_dir: bool,
}

impl Staged {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

/// A handle to something that was deleted, good for [`Store::restore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deleted {
    dir: PathBuf,
    kind: DeletedKind,
    id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeletedKind {
    Note,
    File,
}

impl Deleted {
    pub fn kind(&self) -> DeletedKind {
        self.kind
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

// ---------------------------------------------------------------------------
// On-disk formats
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    version: u32,
    #[serde(default)]
    pinned_notes: Vec<String>,
    #[serde(default)]
    files: BTreeMap<String, FileRecord>,
}

impl Default for IndexFile {
    fn default() -> Self {
        IndexFile {
            version: INDEX_VERSION,
            pinned_notes: Vec::new(),
            files: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileRecord {
    name: String,
    size: u64,
    added_ms: u64,
    #[serde(default)]
    pinned: bool,
    #[serde(default)]
    source: Option<String>,
    /// Set for files that are not copied.
    #[serde(default)]
    link: Option<String>,
    #[serde(default)]
    dir: bool,
    #[serde(skip)]
    missing: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum TrashMeta {
    Note { id: String, pinned: bool },
    File { id: String, record: FileRecord },
}

// ---------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Store {
    shared: Arc<Shared>,
}

#[derive(Debug)]
struct Shared {
    root: PathBuf,
    revision: AtomicU64,
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    notes: HashMap<String, NoteRecord>,
    pinned_notes: HashSet<String>,
    files: BTreeMap<String, FileRecord>,
    warnings: Vec<String>,
}

#[derive(Debug)]
struct NoteRecord {
    body: String,
    mtime_ms: u64,
}

impl Store {
    /// Open (creating if needed) the store rooted at `root`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Store> {
        let root = root.into();
        for sub in ["notes", "files", "trash"] {
            let dir = root.join(sub);
            fs::create_dir_all(&dir).context(|| format!("create {}", dir.display()))?;
        }

        let mut inner = Inner::default();
        match read_index(&root.join("index.json")) {
            Ok(Some(index)) => {
                inner.pinned_notes = index.pinned_notes.into_iter().collect();
                inner.files = index.files;
            }
            Ok(None) => {}
            Err(reason) => {
                let backup = root.join(format!("index.json.corrupt-{}", id::now_ms()));
                let _ = fs::rename(root.join("index.json"), &backup);
                inner.warnings.push(format!(
                    "index.json could not be read ({reason}); it was moved to {} and rebuilt from the files on disk",
                    backup.display()
                ));
            }
        }

        let store = Store {
            shared: Arc::new(Shared {
                root,
                revision: AtomicU64::new(1),
                inner: Mutex::new(inner),
            }),
        };
        {
            let mut inner = store.lock();
            let changed = scan(&store.shared.root, &mut inner);
            if changed {
                save_index(&store.shared.root, &inner)?;
            }
        }
        store.purge_trash(TRASH_RETENTION);
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.shared.root
    }

    /// Increases whenever anything visible changes. Cheap to poll.
    pub fn revision(&self) -> u64 {
        self.shared.revision.load(Ordering::Acquire)
    }

    /// Problems found while opening that the user may want to hear about.
    pub fn take_warnings(&self) -> Vec<String> {
        std::mem::take(&mut self.lock().warnings)
    }

    /// Re-read the directory, picking up changes made behind tsumiba's back
    /// (a sync tool, another process, a file you deleted by hand). Returns
    /// whether anything changed.
    pub fn refresh(&self) -> Result<bool> {
        let mut inner = self.lock();
        let changed = scan(&self.shared.root, &mut inner);
        if changed {
            save_index(&self.shared.root, &inner)?;
            self.bump();
        }
        Ok(changed)
    }

    // ---- notes ------------------------------------------------------------

    /// All notes: pinned first, then most recently changed first.
    pub fn notes(&self) -> Vec<Note> {
        let inner = self.lock();
        let mut notes: Vec<Note> = inner
            .notes
            .iter()
            .map(|(id, rec)| note_from(id, rec, &inner))
            .collect();
        notes.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.updated_ms.cmp(&a.updated_ms))
                .then(b.id.cmp(&a.id))
        });
        notes
    }

    pub fn note(&self, id: &str) -> Option<Note> {
        let inner = self.lock();
        inner.notes.get(id).map(|rec| note_from(id, rec, &inner))
    }

    pub fn add_note(&self, body: &str) -> Result<Note> {
        check_note_size(body)?;
        let id = id::new_id();
        let path = self.note_path(&id);
        write_atomic(&path, body.as_bytes())?;
        let mtime_ms = mtime_ms(&path);
        let mut inner = self.lock();
        inner.notes.insert(
            id.clone(),
            NoteRecord {
                body: body.to_string(),
                mtime_ms,
            },
        );
        self.bump();
        Ok(note_from(&id, &inner.notes[&id], &inner))
    }

    pub fn update_note(&self, id: &str, body: &str) -> Result<Note> {
        check_id(id)?;
        check_note_size(body)?;
        let mut inner = self.lock();
        if !inner.notes.contains_key(id) {
            return Err(Error::NotFound(format!("note {id}")));
        }
        let path = self.note_path(id);
        write_atomic(&path, body.as_bytes())?;
        let mtime_ms = mtime_ms(&path);
        inner.notes.insert(
            id.to_string(),
            NoteRecord {
                body: body.to_string(),
                mtime_ms,
            },
        );
        self.bump();
        Ok(note_from(id, &inner.notes[id], &inner))
    }

    pub fn set_note_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        check_id(id)?;
        let mut inner = self.lock();
        if !inner.notes.contains_key(id) {
            return Err(Error::NotFound(format!("note {id}")));
        }
        let changed = if pinned {
            inner.pinned_notes.insert(id.to_string())
        } else {
            inner.pinned_notes.remove(id)
        };
        if changed {
            save_index(&self.shared.root, &inner)?;
            self.bump();
        }
        Ok(())
    }

    pub fn delete_note(&self, id: &str) -> Result<Deleted> {
        check_id(id)?;
        let mut inner = self.lock();
        if !inner.notes.contains_key(id) {
            return Err(Error::NotFound(format!("note {id}")));
        }
        let pinned = inner.pinned_notes.contains(id);
        let dir = self.new_trash_dir(id)?;
        write_trash_meta(
            &dir,
            &TrashMeta::Note {
                id: id.to_string(),
                pinned,
            },
        )?;
        let payload = dir.join("payload");
        let src = self.note_path(id);
        fs::rename(&src, &payload).context(|| format!("move {} to the trash", src.display()))?;
        inner.notes.remove(id);
        if inner.pinned_notes.remove(id) {
            save_index(&self.shared.root, &inner)?;
        }
        self.bump();
        Ok(Deleted {
            dir,
            kind: DeletedKind::Note,
            id: id.to_string(),
        })
    }

    // ---- files ------------------------------------------------------------

    /// All stored files: pinned first, then most recently added first.
    pub fn files(&self) -> Vec<StoredFile> {
        let inner = self.lock();
        let mut files: Vec<StoredFile> = inner
            .files
            .iter()
            .map(|(id, rec)| self.stored_file(id, rec))
            .collect();
        files.sort_by(|a, b| {
            b.pinned
                .cmp(&a.pinned)
                .then(b.added_ms.cmp(&a.added_ms))
                .then(b.id.cmp(&a.id))
        });
        files
    }

    pub fn file(&self, id: &str) -> Option<StoredFile> {
        let inner = self.lock();
        inner.files.get(id).map(|rec| self.stored_file(id, rec))
    }

    /// Copy `src` into the shelf's directory. Does not touch the index or take
    /// the lock, so it is safe to call from a worker thread while the UI keeps
    /// drawing. Finish with [`commit`](Self::commit).
    ///
    /// A folder is never copied: it is staged as a link instead.
    pub fn stage_copy(&self, src: &Path) -> Result<Staged> {
        let meta = fs::metadata(src).context(|| format!("read {}", src.display()))?;
        if meta.is_dir() {
            return self.stage_link(src);
        }
        if !meta.is_file() {
            return Err(Error::Invalid(format!(
                "{} is not a regular file",
                src.display()
            )));
        }
        let name = sanitize_file_name(&file_name_of(src));
        let id = id::new_id();
        let dir = self.file_dir(&id);
        fs::create_dir_all(&dir).context(|| format!("create {}", dir.display()))?;
        match copy_new(src, &dir.join(&name)) {
            Ok(size) => Ok(Staged {
                id,
                name,
                size,
                link: None,
                is_dir: false,
            }),
            Err(e) => {
                let _ = fs::remove_dir_all(&dir);
                Err(e)
            }
        }
    }

    /// Remember where `src` is without copying it.
    pub fn stage_link(&self, src: &Path) -> Result<Staged> {
        let meta = fs::metadata(src).context(|| format!("read {}", src.display()))?;
        let absolute = std::path::absolute(src).context(|| format!("resolve {}", src.display()))?;
        let link = absolute.to_str().ok_or_else(|| {
            Error::Invalid(format!("{} is not valid UTF-8", absolute.display()))
        })?;
        Ok(Staged {
            id: id::new_id(),
            name: sanitize_file_name(&file_name_of(&absolute)),
            size: if meta.is_dir() { 0 } else { meta.len() },
            link: Some(link.to_string()),
            is_dir: meta.is_dir(),
        })
    }

    /// Write `reader` into the shelf's directory as `name`, refusing more than
    /// `limit` bytes. Used for uploads.
    pub fn stage_reader(&self, name: &str, reader: &mut dyn Read, limit: u64) -> Result<Staged> {
        let name = sanitize_file_name(name);
        let id = id::new_id();
        let dir = self.file_dir(&id);
        fs::create_dir_all(&dir).context(|| format!("create {}", dir.display()))?;
        let dst = dir.join(&name);
        let result = (|| -> Result<u64> {
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&dst)
                .context(|| format!("create {}", dst.display()))?;
            // Read one byte past the limit to tell "exactly at" from "over".
            let mut limited = reader.take(limit.saturating_add(1));
            let n = std::io::copy(&mut limited, &mut out)
                .context(|| format!("write {}", dst.display()))?;
            if n > limit {
                return Err(Error::TooLarge { limit });
            }
            out.sync_all().context(|| format!("flush {}", dst.display()))?;
            Ok(n)
        })();
        match result {
            Ok(size) => Ok(Staged {
                id,
                name,
                size,
                link: None,
                is_dir: false,
            }),
            Err(e) => {
                let _ = fs::remove_dir_all(&dir);
                Err(e)
            }
        }
    }

    /// Add a staged file to the index.
    pub fn commit(&self, staged: Staged, source: Option<String>) -> Result<StoredFile> {
        let record = FileRecord {
            name: staged.name,
            size: staged.size,
            added_ms: id::created_ms(&staged.id).unwrap_or_else(id::now_ms),
            pinned: false,
            source: source.map(|s| truncate_chars(&s, 2048)).filter(|s| !s.is_empty()),
            link: staged.link,
            dir: staged.is_dir,
            missing: false,
        };
        let mut inner = self.lock();
        inner.files.insert(staged.id.clone(), record);
        save_index(&self.shared.root, &inner)?;
        self.bump();
        Ok(self.stored_file(&staged.id, &inner.files[&staged.id]))
    }

    /// Stage and commit in one go. Convenient for the command line; the UI uses
    /// the two halves separately.
    pub fn add_path(&self, src: &Path, mode: StorageMode) -> Result<StoredFile> {
        let staged = match mode {
            StorageMode::Copy => self.stage_copy(src)?,
            StorageMode::Link => self.stage_link(src)?,
        };
        let source = std::path::absolute(src)
            .ok()
            .and_then(|p| p.to_str().map(str::to_string));
        self.commit(staged, source)
    }

    pub fn rename_file(&self, id: &str, new_name: &str) -> Result<()> {
        check_id(id)?;
        let new_name = sanitize_file_name(new_name);
        let mut inner = self.lock();
        let Some(record) = inner.files.get(id).cloned() else {
            return Err(Error::NotFound(format!("file {id}")));
        };
        if record.link.is_none() {
            let dir = self.file_dir(id);
            let from = dir.join(&record.name);
            let to = dir.join(&new_name);
            if from != to {
                if to.exists() {
                    return Err(Error::Invalid(format!("{new_name} already exists")));
                }
                fs::rename(&from, &to).context(|| format!("rename {}", from.display()))?;
            }
        }
        inner.files.get_mut(id).expect("checked above").name = new_name;
        save_index(&self.shared.root, &inner)?;
        self.bump();
        Ok(())
    }

    pub fn set_file_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        check_id(id)?;
        let mut inner = self.lock();
        let Some(record) = inner.files.get_mut(id) else {
            return Err(Error::NotFound(format!("file {id}")));
        };
        if record.pinned != pinned {
            record.pinned = pinned;
            save_index(&self.shared.root, &inner)?;
            self.bump();
        }
        Ok(())
    }

    /// Remove a file from the shelf. A copy goes to the trash; a link only
    /// forgets the location — the original is never touched.
    pub fn delete_file(&self, id: &str) -> Result<Deleted> {
        check_id(id)?;
        let mut inner = self.lock();
        let Some(record) = inner.files.get(id).cloned() else {
            return Err(Error::NotFound(format!("file {id}")));
        };
        let dir = self.new_trash_dir(id)?;
        write_trash_meta(
            &dir,
            &TrashMeta::File {
                id: id.to_string(),
                record: record.clone(),
            },
        )?;
        if record.link.is_none() {
            let src = self.file_dir(id);
            if src.exists() {
                fs::rename(&src, dir.join("payload"))
                    .context(|| format!("move {} to the trash", src.display()))?;
            }
        }
        inner.files.remove(id);
        save_index(&self.shared.root, &inner)?;
        self.bump();
        Ok(Deleted {
            dir,
            kind: DeletedKind::File,
            id: id.to_string(),
        })
    }

    // ---- trash ------------------------------------------------------------

    /// Undo a deletion.
    pub fn restore(&self, deleted: &Deleted) -> Result<()> {
        let meta = read_trash_meta(&deleted.dir)?;
        let mut inner = self.lock();
        match meta {
            TrashMeta::Note { id, pinned } => {
                let dst = self.note_path(&id);
                if dst.exists() {
                    return Err(Error::Invalid(format!("note {id} already exists")));
                }
                fs::rename(deleted.dir.join("payload"), &dst)
                    .context(|| format!("restore {}", dst.display()))?;
                let body = fs::read_to_string(&dst).unwrap_or_default();
                inner.notes.insert(
                    id.clone(),
                    NoteRecord {
                        body,
                        mtime_ms: mtime_ms(&dst),
                    },
                );
                if pinned {
                    inner.pinned_notes.insert(id);
                    save_index(&self.shared.root, &inner)?;
                }
            }
            TrashMeta::File { id, record } => {
                if record.link.is_none() {
                    let dst = self.file_dir(&id);
                    if dst.exists() {
                        return Err(Error::Invalid(format!("file {id} already exists")));
                    }
                    fs::rename(deleted.dir.join("payload"), &dst)
                        .context(|| format!("restore {}", dst.display()))?;
                }
                inner.files.insert(id, record);
                save_index(&self.shared.root, &inner)?;
            }
        }
        let _ = fs::remove_dir_all(&deleted.dir);
        self.bump();
        Ok(())
    }

    /// Permanently remove trash entries older than `max_age`.
    pub fn purge_trash(&self, max_age: Duration) -> usize {
        let trash = self.shared.root.join("trash");
        let Ok(entries) = fs::read_dir(&trash) else {
            return 0;
        };
        let cutoff = id::now_ms().saturating_sub(max_age.as_millis() as u64);
        let mut purged = 0;
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let deleted_ms = name
                .split('_')
                .next()
                .and_then(|ms| ms.parse::<u64>().ok());
            // An entry whose name we cannot read is left alone: it is not ours to delete.
            if deleted_ms.is_some_and(|ms| ms < cutoff) && fs::remove_dir_all(entry.path()).is_ok()
            {
                purged += 1;
            }
        }
        purged
    }

    // ---- helpers ----------------------------------------------------------

    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while holding the lock leaves plain data behind; carrying on
        // is better than taking the whole app down with a poisoned mutex.
        self.shared
            .inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn bump(&self) {
        self.shared.revision.fetch_add(1, Ordering::Release);
    }

    fn note_path(&self, id: &str) -> PathBuf {
        self.shared.root.join("notes").join(format!("{id}.md"))
    }

    fn file_dir(&self, id: &str) -> PathBuf {
        self.shared.root.join("files").join(id)
    }

    fn new_trash_dir(&self, id: &str) -> Result<PathBuf> {
        let dir = self
            .shared
            .root
            .join("trash")
            .join(format!("{}_{}", id::now_ms(), id));
        fs::create_dir_all(&dir).context(|| format!("create {}", dir.display()))?;
        Ok(dir)
    }

    fn stored_file(&self, id: &str, rec: &FileRecord) -> StoredFile {
        let path = match &rec.link {
            Some(link) => PathBuf::from(link),
            None => self.file_dir(id).join(&rec.name),
        };
        StoredFile {
            id: id.to_string(),
            name: rec.name.clone(),
            size: rec.size,
            added_ms: rec.added_ms,
            pinned: rec.pinned,
            source: rec.source.clone(),
            path,
            link: rec.link.is_some(),
            is_dir: rec.dir,
            missing: rec.missing,
        }
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

fn note_from(id: &str, rec: &NoteRecord, inner: &Inner) -> Note {
    Note {
        id: id.to_string(),
        body: rec.body.clone(),
        pinned: inner.pinned_notes.contains(id),
        created_ms: id::created_ms(id).unwrap_or(rec.mtime_ms),
        updated_ms: rec.mtime_ms,
    }
}

fn check_id(id: &str) -> Result<()> {
    if id::is_valid(id) {
        Ok(())
    } else {
        Err(Error::Invalid(format!("{id:?} is not a valid id")))
    }
}

fn check_note_size(body: &str) -> Result<()> {
    if body.len() > MAX_NOTE_BYTES {
        Err(Error::TooLarge {
            limit: MAX_NOTE_BYTES as u64,
        })
    } else {
        Ok(())
    }
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn mtime_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or_else(id::now_ms)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// `Ok(None)` when there is no index yet, `Err` with a reason when there is one
/// that cannot be used.
fn read_index(path: &Path) -> std::result::Result<Option<IndexFile>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let index: IndexFile = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if index.version > INDEX_VERSION {
        return Err(format!(
            "it was written by a newer tsumiba (index version {})",
            index.version
        ));
    }
    Ok(Some(index))
}

fn save_index(root: &Path, inner: &Inner) -> Result<()> {
    let mut pinned: Vec<String> = inner.pinned_notes.iter().cloned().collect();
    pinned.sort();
    let index = IndexFile {
        version: INDEX_VERSION,
        pinned_notes: pinned,
        files: inner.files.clone(),
    };
    let json = serde_json::to_vec_pretty(&index).map_err(|e| Error::Index(e.to_string()))?;
    write_atomic(&root.join("index.json"), &json)
}

fn write_trash_meta(dir: &Path, meta: &TrashMeta) -> Result<()> {
    let json = serde_json::to_vec_pretty(meta).map_err(|e| Error::Index(e.to_string()))?;
    write_atomic(&dir.join("meta.json"), &json)
}

fn read_trash_meta(dir: &Path) -> Result<TrashMeta> {
    let path = dir.join("meta.json");
    let bytes = fs::read(&path).context(|| format!("read {}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| Error::Index(format!("{}: {e}", path.display())))
}

/// Bring `inner` in line with the directory. Returns whether the *index file*
/// or anything the UI shows changed.
fn scan(root: &Path, inner: &mut Inner) -> bool {
    let mut changed = false;

    // Notes: every `<id>.md` is a note.
    let mut seen = HashSet::new();
    if let Ok(entries) = fs::read_dir(root.join("notes")) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(id) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".md"))
                .filter(|id| id::is_valid(id))
                .map(str::to_string)
            else {
                continue;
            };
            let mtime = mtime_ms(&path);
            seen.insert(id.clone());
            let up_to_date = inner.notes.get(&id).is_some_and(|rec| rec.mtime_ms == mtime);
            if up_to_date {
                continue;
            }
            // Not valid UTF-8 means it was not written by us; leave it alone.
            if let Ok(body) = fs::read_to_string(&path) {
                inner.notes.insert(
                    id,
                    NoteRecord {
                        body,
                        mtime_ms: mtime,
                    },
                );
                changed = true;
            }
        }
    }
    let before = inner.notes.len();
    inner.notes.retain(|id, _| seen.contains(id));
    changed |= inner.notes.len() != before;
    let before = inner.pinned_notes.len();
    inner.pinned_notes.retain(|id| seen.contains(id));
    changed |= inner.pinned_notes.len() != before;

    // Files: adopt directories the index does not know about.
    let mut present = HashSet::new();
    if let Ok(entries) = fs::read_dir(root.join("files")) {
        for entry in entries.flatten() {
            let Some(id) = entry
                .file_name()
                .to_str()
                .filter(|id| id::is_valid(id))
                .map(str::to_string)
            else {
                continue;
            };
            let dir = entry.path();
            let Some((name, size)) = first_file_in(&dir) else {
                continue;
            };
            present.insert(id.clone());
            if !inner.files.contains_key(&id) {
                inner.files.insert(
                    id.clone(),
                    FileRecord {
                        name,
                        size,
                        added_ms: id::created_ms(&id).unwrap_or_else(id::now_ms),
                        pinned: false,
                        source: None,
                        link: None,
                        dir: false,
                        missing: false,
                    },
                );
                changed = true;
            }
        }
    }
    // Copies whose payload vanished are dropped; links only get flagged.
    let before = inner.files.len();
    inner
        .files
        .retain(|id, rec| rec.link.is_some() || present.contains(id));
    changed |= inner.files.len() != before;
    for rec in inner.files.values_mut() {
        if let Some(link) = &rec.link {
            let missing = !Path::new(link).exists();
            if rec.missing != missing {
                rec.missing = missing;
                changed = true;
            }
        }
    }
    changed
}

fn first_file_in(dir: &Path) -> Option<(String, u64)> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .collect();
    names.sort_by_key(|e| e.file_name());
    let entry = names.into_iter().next()?;
    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
    Some((entry.file_name().to_string_lossy().into_owned(), size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::time::SystemTime;

    fn open() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().join("data")).unwrap();
        (dir, store)
    }

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    // ---- notes ----

    #[test]
    fn notes_are_created_updated_and_listed() {
        let (_tmp, store) = open();
        let a = store.add_note("first").unwrap();
        let b = store.add_note("second").unwrap();
        assert_eq!(store.notes().len(), 2);

        let updated = store.update_note(&a.id, "first, edited").unwrap();
        assert_eq!(updated.body, "first, edited");
        assert_eq!(store.note(&a.id).unwrap().body, "first, edited");
        assert_eq!(store.note(&b.id).unwrap().body, "second");
        assert!(store.note("nope").is_none());
    }

    #[test]
    fn notes_survive_reopening() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("data");
        let id = {
            let store = Store::open(&root).unwrap();
            let note = store.add_note("日本語のメモ\nsecond line").unwrap();
            store.set_note_pinned(&note.id, true).unwrap();
            note.id
        };
        let store = Store::open(&root).unwrap();
        let note = store.note(&id).unwrap();
        assert_eq!(note.body, "日本語のメモ\nsecond line");
        assert!(note.pinned);
    }

    #[test]
    fn pinned_notes_sort_first() {
        let (_tmp, store) = open();
        let old = store.add_note("old").unwrap();
        std::thread::sleep(Duration::from_millis(5));
        let new = store.add_note("new").unwrap();
        assert_eq!(store.notes()[0].id, new.id);
        store.set_note_pinned(&old.id, true).unwrap();
        assert_eq!(store.notes()[0].id, old.id);
    }

    #[test]
    fn note_title_and_snippet() {
        let note = |body: &str| Note {
            id: id::new_id(),
            body: body.to_string(),
            pinned: false,
            created_ms: 0,
            updated_ms: 0,
        };
        assert_eq!(note("# Heading\n\nbody text\nmore").title(), "Heading");
        assert_eq!(note("# Heading\n\nbody text\nmore").snippet(), "body text more");
        assert_eq!(note("\n\n  hello  ").title(), "hello");
        assert_eq!(note("").title(), "");
        assert_eq!(note("only title").snippet(), "");
        let long = "x".repeat(200);
        assert_eq!(note(&long).title().chars().count(), 80);
        assert!(note(&long).title().ends_with('…'));
    }

    #[test]
    fn note_search_is_case_insensitive() {
        let note = Note {
            id: id::new_id(),
            body: "Buy Milk".into(),
            pinned: false,
            created_ms: 0,
            updated_ms: 0,
        };
        assert!(note.matches("milk"));
        assert!(note.matches("  BUY "));
        assert!(note.matches(""));
        assert!(!note.matches("eggs"));
    }

    #[test]
    fn oversized_notes_are_refused() {
        let (_tmp, store) = open();
        let huge = "x".repeat(MAX_NOTE_BYTES + 1);
        assert!(matches!(store.add_note(&huge), Err(Error::TooLarge { .. })));
    }

    #[test]
    fn ids_from_outside_are_validated() {
        let (_tmp, store) = open();
        for bad in ["../x", "", "a/b", "..\\x"] {
            assert!(matches!(store.update_note(bad, "x"), Err(Error::Invalid(_))), "{bad}");
            assert!(matches!(store.delete_note(bad), Err(Error::Invalid(_))), "{bad}");
            assert!(matches!(store.delete_file(bad), Err(Error::Invalid(_))), "{bad}");
        }
        let missing = id::new_id();
        assert!(matches!(store.update_note(&missing, "x"), Err(Error::NotFound(_))));
    }

    #[test]
    fn deleted_notes_can_be_restored() {
        let (_tmp, store) = open();
        let note = store.add_note("keep me").unwrap();
        store.set_note_pinned(&note.id, true).unwrap();
        let deleted = store.delete_note(&note.id).unwrap();
        assert_eq!(deleted.kind(), DeletedKind::Note);
        assert!(store.note(&note.id).is_none());
        assert!(store.notes().is_empty());

        store.restore(&deleted).unwrap();
        let back = store.note(&note.id).unwrap();
        assert_eq!(back.body, "keep me");
        assert!(back.pinned, "pinned state is restored too");
    }

    // ---- files ----

    #[test]
    fn copying_a_file_keeps_a_private_copy() {
        let (tmp, store) = open();
        let src = write(tmp.path(), "report.txt", "hello");
        let file = store.add_path(&src, StorageMode::Copy).unwrap();
        assert_eq!(file.name, "report.txt");
        assert_eq!(file.size, 5);
        assert!(!file.link);
        assert!(file.path.starts_with(store.root()));

        fs::remove_file(&src).unwrap();
        assert_eq!(fs::read_to_string(&file.path).unwrap(), "hello");
        assert_eq!(store.files().len(), 1);
    }

    #[test]
    fn same_name_twice_gets_two_entries() {
        let (tmp, store) = open();
        let src = write(tmp.path(), "a.txt", "one");
        let first = store.add_path(&src, StorageMode::Copy).unwrap();
        fs::write(&src, "two").unwrap();
        let second = store.add_path(&src, StorageMode::Copy).unwrap();
        assert_ne!(first.id, second.id);
        assert_eq!(fs::read_to_string(first.path).unwrap(), "one");
        assert_eq!(fs::read_to_string(second.path).unwrap(), "two");
    }

    #[test]
    fn linking_does_not_copy_and_never_deletes_the_original() {
        let (tmp, store) = open();
        let src = write(tmp.path(), "big.iso", "pretend this is large");
        let file = store.add_path(&src, StorageMode::Link).unwrap();
        assert!(file.link);
        assert_eq!(file.path, std::path::absolute(&src).unwrap());
        assert!(!store.root().join("files").join(&file.id).exists());

        store.delete_file(&file.id).unwrap();
        assert!(src.exists(), "the original must survive a delete");
        assert!(store.files().is_empty());
    }

    #[test]
    fn folders_are_always_links() {
        let (tmp, store) = open();
        let folder = tmp.path().join("project");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("x"), "1").unwrap();
        let entry = store.add_path(&folder, StorageMode::Copy).unwrap();
        assert!(entry.link && entry.is_dir);
        assert_eq!(entry.name, "project");
    }

    #[test]
    fn missing_sources_are_reported() {
        let (tmp, store) = open();
        let err = store
            .add_path(&tmp.path().join("nope"), StorageMode::Copy)
            .unwrap_err();
        assert!(matches!(err, Error::Io { .. }), "{err}");
        assert!(store.files().is_empty());
    }

    #[test]
    fn a_link_whose_target_disappears_is_flagged_not_dropped() {
        let (tmp, store) = open();
        let src = write(tmp.path(), "gone.txt", "x");
        let file = store.add_path(&src, StorageMode::Link).unwrap();
        assert!(!file.missing);
        fs::remove_file(&src).unwrap();
        assert!(store.refresh().unwrap());
        let file = store.file(&file.id).unwrap();
        assert!(file.missing);
    }

    #[test]
    fn deleted_copies_can_be_restored() {
        let (tmp, store) = open();
        let src = write(tmp.path(), "photo.png", "png-bytes");
        let file = store.add_path(&src, StorageMode::Copy).unwrap();
        store.set_file_pinned(&file.id, true).unwrap();
        let deleted = store.delete_file(&file.id).unwrap();
        assert!(!file.path.exists());
        assert!(store.files().is_empty());

        store.restore(&deleted).unwrap();
        let back = store.file(&file.id).unwrap();
        assert!(back.pinned);
        assert_eq!(fs::read_to_string(back.path).unwrap(), "png-bytes");
    }

    #[test]
    fn rename_moves_the_stored_copy() {
        let (tmp, store) = open();
        let src = write(tmp.path(), "old.txt", "x");
        let file = store.add_path(&src, StorageMode::Copy).unwrap();
        store.rename_file(&file.id, "new/name?.txt").unwrap();
        let renamed = store.file(&file.id).unwrap();
        assert_eq!(renamed.name, "name_.txt");
        assert!(renamed.path.exists());
        assert!(!file.path.exists());
    }

    #[test]
    fn upload_from_a_reader_respects_the_limit() {
        let (_tmp, store) = open();
        let staged = store
            .stage_reader("../../evil.txt", &mut &b"12345"[..], 5)
            .unwrap();
        assert_eq!(staged.name(), "evil.txt");
        let file = store.commit(staged, Some("https://example.com/a".into())).unwrap();
        assert_eq!(file.size, 5);
        assert_eq!(file.source.as_deref(), Some("https://example.com/a"));
        assert!(file.path.starts_with(store.root()));

        let err = store
            .stage_reader("big.bin", &mut &b"123456"[..], 5)
            .unwrap_err();
        assert!(matches!(err, Error::TooLarge { limit: 5 }), "{err}");
        // The partial file was cleaned up.
        let dirs = fs::read_dir(store.root().join("files")).unwrap().count();
        assert_eq!(dirs, 1);
    }

    #[test]
    fn staging_alone_does_not_change_the_shelf() {
        let (tmp, store) = open();
        let src = write(tmp.path(), "a.txt", "x");
        let before = store.revision();
        let _staged = store.stage_copy(&src).unwrap();
        assert_eq!(store.revision(), before);
        assert!(store.files().is_empty());
    }

    // ---- recovery & refresh ----

    #[test]
    fn orphans_on_disk_are_adopted() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("data");
        let (note_id, file_id) = {
            let store = Store::open(&root).unwrap();
            let src = write(tmp.path(), "a.txt", "payload");
            let note = store.add_note("note").unwrap();
            let file = store.add_path(&src, StorageMode::Copy).unwrap();
            (note.id, file.id)
        };
        // Lose the index entirely.
        fs::remove_file(root.join("index.json")).unwrap();
        let store = Store::open(&root).unwrap();
        assert_eq!(store.note(&note_id).unwrap().body, "note");
        let file = store.file(&file_id).unwrap();
        assert_eq!(file.name, "a.txt");
        assert_eq!(file.size, 7);
    }

    #[test]
    fn a_corrupt_index_is_set_aside_and_rebuilt() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("data");
        let id = {
            let store = Store::open(&root).unwrap();
            store.add_note("still here").unwrap().id
        };
        fs::write(root.join("index.json"), "{ not json").unwrap();
        let store = Store::open(&root).unwrap();
        assert_eq!(store.note(&id).unwrap().body, "still here");
        let warnings = store.take_warnings();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("index.json"));
        assert!(fs::read_dir(&root).unwrap().flatten().any(|e| e
            .file_name()
            .to_string_lossy()
            .starts_with("index.json.corrupt-")));
        assert!(store.take_warnings().is_empty(), "warnings are reported once");
    }

    #[test]
    fn an_index_from_the_future_is_not_overwritten_silently() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("data");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("index.json"), r#"{"version": 99}"#).unwrap();
        let store = Store::open(&root).unwrap();
        assert_eq!(store.take_warnings().len(), 1);
    }

    #[test]
    fn refresh_sees_changes_made_behind_our_back() {
        let (_tmp, store) = open();
        let note = store.add_note("v1").unwrap();
        let before = store.revision();
        assert!(!store.refresh().unwrap(), "nothing changed yet");
        assert_eq!(store.revision(), before);

        // Someone else edits, adds and deletes.
        let path = store.root().join("notes").join(format!("{}.md", note.id));
        fs::write(&path, "v2 from elsewhere").unwrap();
        // Make sure the mtime differs even on coarse filesystems.
        let later = SystemTime::now() + Duration::from_secs(5);
        fs::File::options().write(true).open(&path).unwrap().set_modified(later).unwrap();
        let extra = id::new_id();
        fs::write(store.root().join("notes").join(format!("{extra}.md")), "added").unwrap();

        assert!(store.refresh().unwrap());
        assert_eq!(store.note(&note.id).unwrap().body, "v2 from elsewhere");
        assert_eq!(store.note(&extra).unwrap().body, "added");

        fs::remove_file(&path).unwrap();
        assert!(store.refresh().unwrap());
        assert!(store.note(&note.id).is_none());
    }

    #[test]
    fn stray_files_in_notes_are_ignored() {
        let (_tmp, store) = open();
        fs::write(store.root().join("notes").join("README.txt"), "hi").unwrap();
        fs::write(store.root().join("notes").join("not-an-id.md"), "hi").unwrap();
        assert!(!store.refresh().unwrap());
        assert!(store.notes().is_empty());
    }

    // ---- trash ----

    #[test]
    fn old_trash_is_purged_and_new_trash_is_kept() {
        let (_tmp, store) = open();
        let note = store.add_note("x").unwrap();
        store.delete_note(&note.id).unwrap();
        assert_eq!(store.purge_trash(TRASH_RETENTION), 0, "fresh entries stay");
        assert_eq!(store.purge_trash(Duration::ZERO), 1, "everything is older than zero");
        assert_eq!(fs::read_dir(store.root().join("trash")).unwrap().count(), 0);
    }

    #[test]
    fn foreign_entries_in_trash_are_left_alone() {
        let (_tmp, store) = open();
        let stranger = store.root().join("trash").join("do-not-touch");
        fs::create_dir(&stranger).unwrap();
        assert_eq!(store.purge_trash(Duration::ZERO), 0);
        assert!(stranger.exists());
    }

    // ---- handles ----

    #[test]
    fn clones_share_state_across_threads() {
        let (_tmp, store) = open();
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let store = store.clone();
                std::thread::spawn(move || store.add_note(&format!("note {i}")).unwrap().id)
            })
            .collect();
        let ids: HashSet<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(ids.len(), 4);
        assert_eq!(store.notes().len(), 4);
    }

    #[test]
    fn revision_moves_on_every_visible_change() {
        let (tmp, store) = open();
        let mut last = store.revision();
        let mut check = |what: &str| {
            let now = store.revision();
            assert!(now > last, "{what} did not bump the revision");
            last = now;
        };
        let note = store.add_note("a").unwrap();
        check("add_note");
        store.update_note(&note.id, "b").unwrap();
        check("update_note");
        store.set_note_pinned(&note.id, true).unwrap();
        check("set_note_pinned");
        let src = write(tmp.path(), "f", "x");
        let file = store.add_path(&src, StorageMode::Copy).unwrap();
        check("add_path");
        store.set_file_pinned(&file.id, true).unwrap();
        check("set_file_pinned");
        store.delete_file(&file.id).unwrap();
        check("delete_file");
        store.delete_note(&note.id).unwrap();
        check("delete_note");
    }
}
