//! The local HTTP API (`/v1`) used by the command line and the browser extension.
//!
//! It listens on `127.0.0.1` only, and because any web page can make your
//! browser talk to `127.0.0.1`, loopback alone is not protection. Three layers are:
//!
//! 1. **Host** must be `127.0.0.1:<port>`, `localhost:<port>` or `[::1]:<port>`
//!    (defeats DNS rebinding, where an attacker's domain resolves to 127.0.0.1).
//! 2. **Origin**, when present, must be a browser-extension origin. Ordinary web
//!    pages are refused before anything is read.
//! 3. A **bearer token** that only the app's own files and the extension's
//!    storage contain. `GET /v1/ping` is the one endpoint without it.
//!
//! The full contract is documented in `docs-site/en/src/api.md`.

use std::io::Read;
use std::net::SocketAddr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tiny_http::{Header, Method, Request, Response};
use tsumiba_core::api::{
    self, ErrorBody, FileDto, ImportFailure, ImportRequest, ImportResult, NewNote, NoteDto,
    NotePatch, Ping, Status, WindowAction, WindowRequest,
};
use tsumiba_core::config::{Config, StorageMode};
use tsumiba_core::store::{MAX_NOTE_BYTES, Note, Store, StoredFile};
use tsumiba_core::{Error, token};

use crate::control::{Command, Control};

pub type SharedConfig = Arc<RwLock<Config>>;

const WORKERS: usize = 4;
/// Leave room for JSON escaping around the largest allowed note.
const MAX_JSON_BYTES: u64 = MAX_NOTE_BYTES as u64 + 64 * 1024;

#[derive(Clone)]
pub struct ApiContext {
    pub store: Store,
    pub control: Control,
    pub token: String,
    pub config: SharedConfig,
}

pub struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl Server {
    /// Bind `127.0.0.1:port` (`0` picks a free port) and start answering.
    pub fn start(ctx: ApiContext, port: u16) -> Result<Server, String> {
        let server = tiny_http::Server::http(SocketAddr::from(([127, 0, 0, 1], port)))
            .map_err(|e| e.to_string())?;
        let port = server
            .server_addr()
            .to_ip()
            .map(|a| a.port())
            .ok_or("the server is not listening on an IP address")?;
        let server = Arc::new(server);
        let stop = Arc::new(AtomicBool::new(false));

        let workers = (0..WORKERS)
            .filter_map(|n| {
                let server = server.clone();
                let ctx = ctx.clone();
                let stop = stop.clone();
                std::thread::Builder::new()
                    .name(format!("tsumiba-api-{n}"))
                    .spawn(move || {
                        while !stop.load(Ordering::Relaxed) {
                            match server.recv_timeout(Duration::from_millis(250)) {
                                Ok(Some(request)) => serve(&ctx, port, request),
                                Ok(None) => {}
                                Err(e) => {
                                    log::warn!("the API stopped accepting requests: {e}");
                                    break;
                                }
                            }
                        }
                    })
                    .ok()
            })
            .collect::<Vec<_>>();
        if workers.is_empty() {
            return Err("could not start any API worker thread".into());
        }
        Ok(Server {
            port,
            stop,
            workers,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Stop accepting requests and wait for the workers.
    pub fn shutdown(self) {
        self.stop.store(true, Ordering::Relaxed);
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Request handling
// ---------------------------------------------------------------------------

/// What to send back. Built by the handlers, turned into a `tiny_http`
/// response in one place so headers (CORS, caching) cannot be forgotten.
struct Reply {
    status: u16,
    body: Vec<u8>,
    allow_origin: Option<String>,
    preflight: bool,
}

impl Reply {
    fn json<T: Serialize>(status: u16, value: &T) -> Reply {
        Reply {
            status,
            body: serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec()),
            allow_origin: None,
            preflight: false,
        }
    }

    fn empty(status: u16) -> Reply {
        Reply {
            status,
            body: Vec::new(),
            allow_origin: None,
            preflight: false,
        }
    }

    fn error(status: u16, message: impl Into<String>) -> Reply {
        Reply::json(
            status,
            &ErrorBody {
                error: message.into(),
            },
        )
    }

    fn from_core(error: &Error) -> Reply {
        match error {
            Error::NotFound(_) => Reply::error(404, error.to_string()),
            Error::Invalid(_) => Reply::error(400, error.to_string()),
            Error::TooLarge { .. } => Reply::error(413, error.to_string()),
            _ => {
                log::error!("API request failed: {error}");
                Reply::error(500, error.to_string())
            }
        }
    }
}

fn serve(ctx: &ApiContext, port: u16, mut request: Request) {
    let origin = header(&request, "origin");
    let reply =
        catch_unwind(AssertUnwindSafe(|| route(ctx, port, &mut request))).unwrap_or_else(|_| {
            log::error!("a request handler panicked");
            Reply::error(500, "internal error")
        });
    let reply = with_cors(reply, origin.as_deref());
    let _ = request.respond(into_response(reply));
}

fn into_response(reply: Reply) -> Response<std::io::Cursor<Vec<u8>>> {
    let status = reply.status;
    let mut response = Response::from_data(reply.body).with_status_code(status);
    let mut add = |name: &str, value: &str| {
        if let Ok(h) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response.add_header(h);
        }
    };
    add("Content-Type", "application/json; charset=utf-8");
    add("Cache-Control", "no-store");
    add("X-Content-Type-Options", "nosniff");
    if let Some(origin) = &reply.allow_origin {
        add("Access-Control-Allow-Origin", origin);
        add("Vary", "Origin");
        if reply.preflight {
            add(
                "Access-Control-Allow-Methods",
                "GET, POST, PATCH, DELETE, OPTIONS",
            );
            add(
                "Access-Control-Allow-Headers",
                "authorization, content-type, x-tsumiba-name, x-tsumiba-source",
            );
            add("Access-Control-Max-Age", "600");
        }
    }
    response
}

/// Attach CORS permission to replies for extension origins (and only those).
fn with_cors(mut reply: Reply, origin: Option<&str>) -> Reply {
    if let Some(origin) = origin.filter(|o| origin_allowed(o)) {
        reply.allow_origin = Some(origin.to_string());
    }
    reply
}

fn route(ctx: &ApiContext, port: u16, request: &mut Request) -> Reply {
    // Layer 1: the Host header.
    match header(request, "host") {
        Some(host) if host_allowed(&host, port) => {}
        _ => return Reply::error(403, "unexpected Host header"),
    }
    // Layer 2: the Origin header.
    let origin = header(request, "origin");
    if let Some(origin) = &origin
        && !origin_allowed(origin)
    {
        return Reply::error(403, "this origin is not allowed");
    }

    let method = request.method().clone();
    let url = request.url().to_string();
    let (path, query) = split_query(&url);

    if method == Method::Options {
        // A preflight for an extension origin; refused above for anything else.
        let mut reply = Reply::empty(204);
        reply.preflight = true;
        return reply;
    }

    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    let route = match segments.as_slice() {
        ["v1", rest @ ..] => rest,
        _ => return Reply::error(404, "not found"),
    };

    // The one open endpoint: lets a client tell tsumiba from another program.
    if method == Method::Get && route == ["ping"] {
        return Reply::json(200, &Ping::current());
    }

    // Layer 3: the token.
    let given = header(request, "authorization")
        .and_then(|v| v.strip_prefix("Bearer ").map(|t| t.trim().to_string()));
    match given {
        Some(given) if token::matches(&ctx.token, &given) => {}
        _ => return Reply::error(401, "missing or wrong token"),
    }

    match (&method, route) {
        (Method::Get, ["status"]) => Reply::json(
            200,
            &Status {
                notes: ctx.store.notes().len(),
                files: ctx.store.files().len(),
                visible: ctx.control.is_visible(),
            },
        ),
        (Method::Post, ["window"]) => window(ctx, request),
        (Method::Post, ["quit"]) => {
            ctx.control.send(Command::Quit);
            Reply::empty(204)
        }
        (Method::Get, ["notes"]) => list_notes(ctx, query),
        (Method::Post, ["notes"]) => create_note(ctx, request),
        (Method::Get, ["notes", id]) => match ctx.store.note(id) {
            Some(note) => Reply::json(200, &note_dto(&note)),
            None => Reply::error(404, "no such note"),
        },
        (Method::Patch, ["notes", id]) => patch_note(ctx, id, request),
        (Method::Delete, ["notes", id]) => {
            let result = ctx.store.delete_note(id);
            changed(ctx, result.map(|_| Reply::empty(204)))
        }
        (Method::Get, ["files"]) => {
            let files: Vec<FileDto> = ctx.store.files().iter().map(file_dto).collect();
            Reply::json(200, &files)
        }
        (Method::Post, ["files"]) => upload_file(ctx, request),
        (Method::Post, ["files", "import"]) => import_files(ctx, request),
        (Method::Delete, ["files", id]) => {
            let result = ctx.store.delete_file(id);
            changed(ctx, result.map(|_| Reply::empty(204)))
        }
        _ => Reply::error(404, "not found"),
    }
}

/// Wake the window after a successful change so it redraws, and map errors.
fn changed(ctx: &ApiContext, result: Result<Reply, Error>) -> Reply {
    match result {
        Ok(reply) => {
            ctx.control.wake();
            reply
        }
        Err(e) => Reply::from_core(&e),
    }
}

fn window(ctx: &ApiContext, request: &mut Request) -> Reply {
    let body: WindowRequest = match read_json(request, 4096) {
        Ok(body) => body,
        Err(reply) => return reply,
    };
    ctx.control.send(match body.action {
        WindowAction::Show => Command::Show,
        WindowAction::Hide => Command::Hide,
        WindowAction::Toggle => Command::Toggle,
    });
    Reply::empty(204)
}

fn list_notes(ctx: &ApiContext, query: &str) -> Reply {
    let params = parse_query(query);
    let q = params.get("q").map(String::as_str).unwrap_or("");
    let limit = params
        .get("limit")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(usize::MAX);
    let notes: Vec<NoteDto> = ctx
        .store
        .notes()
        .iter()
        .filter(|n| n.matches(q))
        .take(limit)
        .map(note_dto)
        .collect();
    Reply::json(200, &notes)
}

fn create_note(ctx: &ApiContext, request: &mut Request) -> Reply {
    let body: NewNote = match read_json(request, MAX_JSON_BYTES) {
        Ok(body) => body,
        Err(reply) => return reply,
    };
    let result = ctx.store.add_note(&body.text).and_then(|note| {
        if body.pinned {
            ctx.store.set_note_pinned(&note.id, true)?;
        }
        Ok(ctx.store.note(&note.id).unwrap_or(note))
    });
    changed(ctx, result.map(|note| Reply::json(201, &note_dto(&note))))
}

fn patch_note(ctx: &ApiContext, id: &str, request: &mut Request) -> Reply {
    let patch: NotePatch = match read_json(request, MAX_JSON_BYTES) {
        Ok(patch) => patch,
        Err(reply) => return reply,
    };
    let result = (|| {
        if let Some(text) = &patch.text {
            ctx.store.update_note(id, text)?;
        }
        if let Some(pinned) = patch.pinned {
            ctx.store.set_note_pinned(id, pinned)?;
        }
        ctx.store
            .note(id)
            .ok_or_else(|| Error::NotFound(format!("note {id}")))
    })();
    changed(ctx, result.map(|note| Reply::json(200, &note_dto(&note))))
}

fn upload_file(ctx: &ApiContext, request: &mut Request) -> Reply {
    let limit = ctx
        .config
        .read()
        .map(|c| u64::from(c.storage.max_upload_mb) * 1024 * 1024)
        .unwrap_or(512 * 1024 * 1024);
    if request.body_length().is_some_and(|n| n as u64 > limit) {
        return Reply::from_core(&Error::TooLarge { limit });
    }

    let mut name = header(request, api::HEADER_NAME.to_ascii_lowercase().as_str())
        .map(|v| api::decode_header(&v))
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "file".to_string());
    let source = header(request, api::HEADER_SOURCE.to_ascii_lowercase().as_str())
        .map(|v| api::decode_header(&v))
        .filter(|v| !v.trim().is_empty());
    if std::path::Path::new(&name).extension().is_none()
        && let Some(ext) = header(request, "content-type").and_then(|ct| extension_for_mime(&ct))
    {
        name.push('.');
        name.push_str(ext);
    }

    let result = ctx
        .store
        .stage_reader(&name, request.as_reader(), limit)
        .and_then(|staged| ctx.store.commit(staged, source));
    changed(ctx, result.map(|file| Reply::json(201, &file_dto(&file))))
}

fn import_files(ctx: &ApiContext, request: &mut Request) -> Reply {
    let body: ImportRequest = match read_json(request, MAX_JSON_BYTES) {
        Ok(body) => body,
        Err(reply) => return reply,
    };
    let mode = if body.link {
        StorageMode::Link
    } else {
        ctx.config
            .read()
            .map(|c| c.storage.mode)
            .unwrap_or(StorageMode::Copy)
    };

    let mut result = ImportResult::default();
    for path in body.paths {
        let candidate = PathBuf::from(&path);
        let outcome = if candidate.is_absolute() {
            ctx.store.add_path(&candidate, mode)
        } else {
            Err(Error::Invalid("the path must be absolute".into()))
        };
        match outcome {
            Ok(file) => result.added.push(file_dto(&file)),
            Err(e) => result.failed.push(ImportFailure {
                path,
                error: e.to_string(),
            }),
        }
    }
    ctx.control.wake();
    Reply::json(200, &result)
}

/// Read a JSON body of at most `limit` bytes, or the error reply to send.
fn read_json<T: DeserializeOwned>(request: &mut Request, limit: u64) -> Result<T, Reply> {
    if request.body_length().is_some_and(|n| n as u64 > limit) {
        return Err(Reply::from_core(&Error::TooLarge { limit }));
    }
    let mut bytes = Vec::new();
    request
        .as_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| Reply::error(400, format!("could not read the request body: {e}")))?;
    if bytes.len() as u64 > limit {
        return Err(Reply::from_core(&Error::TooLarge { limit }));
    }
    serde_json::from_slice(&bytes).map_err(|e| Reply::error(400, format!("invalid JSON: {e}")))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn header(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str().to_string())
}

/// Only extension origins may call the API from a browser.
pub fn origin_allowed(origin: &str) -> bool {
    [
        "chrome-extension://",
        "moz-extension://",
        "safari-web-extension://",
    ]
    .iter()
    .any(|scheme| origin.starts_with(scheme) && origin.len() > scheme.len())
}

pub fn host_allowed(host: &str, port: u16) -> bool {
    host == format!("127.0.0.1:{port}")
        || host == format!("localhost:{port}")
        || host == format!("[::1]:{port}")
}

fn split_query(url: &str) -> (&str, &str) {
    url.split_once('?').unwrap_or((url, ""))
}

fn parse_query(query: &str) -> std::collections::HashMap<String, String> {
    query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (
                api::decode_header(&key.replace('+', " ")),
                api::decode_header(&value.replace('+', " ")),
            )
        })
        .collect()
}

fn extension_for_mime(content_type: &str) -> Option<&'static str> {
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    Some(match mime.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/avif" => "avif",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "text/html" => "html",
        "application/json" => "json",
        "application/zip" => "zip",
        _ => return None,
    })
}

pub fn note_dto(note: &Note) -> NoteDto {
    NoteDto {
        id: note.id.clone(),
        title: note.title(),
        body: note.body.clone(),
        pinned: note.pinned,
        created_ms: note.created_ms,
        updated_ms: note.updated_ms,
    }
}

pub fn file_dto(file: &StoredFile) -> FileDto {
    FileDto {
        id: file.id.clone(),
        name: file.name.clone(),
        size: file.size,
        added_ms: file.added_ms,
        pinned: file.pinned,
        source: file.source.clone(),
        link: file.link,
        path: file.path.to_string_lossy().into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_extension_origins_are_allowed() {
        assert!(origin_allowed("chrome-extension://abcdefghijklmnop"));
        assert!(origin_allowed("moz-extension://1234-5678"));
        assert!(origin_allowed("safari-web-extension://x"));
        for bad in [
            "",
            "null",
            "https://example.com",
            "http://127.0.0.1:47821",
            "http://localhost",
            "chrome-extension://",
            "chrome-extension.evil.com",
        ] {
            assert!(!origin_allowed(bad), "{bad:?}");
        }
    }

    #[test]
    fn host_must_name_loopback_and_the_right_port() {
        assert!(host_allowed("127.0.0.1:47821", 47821));
        assert!(host_allowed("localhost:47821", 47821));
        assert!(host_allowed("[::1]:47821", 47821));
        for bad in [
            "127.0.0.1:1",
            "evil.example.com:47821",
            "evil.example.com",
            "127.0.0.1",
            "127.0.0.1.evil.com:47821",
            "",
        ] {
            assert!(!host_allowed(bad, 47821), "{bad:?}");
        }
    }

    #[test]
    fn query_parsing() {
        let q = parse_query("q=hello+world&limit=5&flag&x=%E6%97%A5");
        assert_eq!(q["q"], "hello world");
        assert_eq!(q["limit"], "5");
        assert_eq!(q["flag"], "");
        assert_eq!(q["x"], "日");
        assert!(parse_query("").is_empty());
        assert_eq!(split_query("/v1/notes?q=a"), ("/v1/notes", "q=a"));
        assert_eq!(split_query("/v1/notes"), ("/v1/notes", ""));
    }

    #[test]
    fn mime_types_map_to_extensions() {
        assert_eq!(extension_for_mime("image/png"), Some("png"));
        assert_eq!(
            extension_for_mime("image/jpeg; charset=binary"),
            Some("jpg")
        );
        assert_eq!(extension_for_mime("IMAGE/WEBP"), Some("webp"));
        assert_eq!(extension_for_mime("application/x-unknown"), None);
        assert_eq!(extension_for_mime(""), None);
    }
}
