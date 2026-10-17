//! The HTTP API, end to end: a real server on a real socket.

mod common;

use std::sync::mpsc::Receiver;
use std::sync::{Arc, RwLock};

use common::Request;
use tsumiba::control::{Command, Control};
use tsumiba::server::{ApiContext, Server};
use tsumiba_core::api::encode_header;
use tsumiba_core::{Config, Store};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const EXTENSION: &str = "chrome-extension://abcdefghijklmnopabcdefghijklmnop";

struct Fixture {
    port: u16,
    store: Store,
    rx: Receiver<Command>,
    config: Arc<RwLock<Config>>,
    _server: Server,
    _dir: tempfile::TempDir,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("data")).unwrap();
    let (control, rx) = Control::new();
    let config = Arc::new(RwLock::new(Config::default()));
    let server = Server::start(
        ApiContext {
            store: store.clone(),
            control,
            token: TOKEN.to_string(),
            config: config.clone(),
        },
        0,
    )
    .unwrap();
    Fixture {
        port: server.port(),
        store,
        rx,
        config,
        _server: server,
        _dir: dir,
    }
}

impl Fixture {
    fn get(&self, path: &str) -> common::Response {
        Request::new("GET", path).bearer(TOKEN).send(self.port)
    }

    fn post_json(&self, path: &str, body: &str) -> common::Response {
        Request::new("POST", path)
            .bearer(TOKEN)
            .json(body)
            .send(self.port)
    }
}

// ---- authentication ---------------------------------------------------------

#[test]
fn ping_needs_no_token_and_identifies_tsumiba() {
    let f = fixture();
    let r = Request::new("GET", "/v1/ping").send(f.port);
    assert_eq!(r.status, 200);
    assert_eq!(r.json()["app"], "tsumiba");
    assert!(r.json()["version"].is_string());
}

#[test]
fn everything_else_needs_the_token() {
    let f = fixture();
    for (method, path) in [
        ("GET", "/v1/status"),
        ("GET", "/v1/notes"),
        ("POST", "/v1/notes"),
        ("GET", "/v1/files"),
        ("POST", "/v1/window"),
        ("POST", "/v1/quit"),
        ("DELETE", "/v1/notes/0123456789abcdef0123"),
    ] {
        let r = Request::new(method, path).json("{}").send(f.port);
        assert_eq!(r.status, 401, "{method} {path} without a token");
        assert!(r.json()["error"].is_string());
    }
    assert!(f.store.notes().is_empty(), "nothing was created");
}

#[test]
fn a_wrong_token_is_refused() {
    let f = fixture();
    for bad in [
        "wrong",
        "",
        &TOKEN[..63],
        &format!("{TOKEN}0"),
        &TOKEN.to_uppercase(),
    ] {
        let r = Request::new("GET", "/v1/status").bearer(bad).send(f.port);
        assert_eq!(r.status, 401, "token {bad:?}");
    }
    // Wrong scheme.
    let r = Request::new("GET", "/v1/status")
        .header("Authorization", format!("Basic {TOKEN}"))
        .send(f.port);
    assert_eq!(r.status, 401);
}

#[test]
fn the_right_token_gets_status() {
    let f = fixture();
    f.store.add_note("one").unwrap();
    let r = f.get("/v1/status");
    assert_eq!(r.status, 200);
    assert_eq!(r.json()["notes"], 1);
    assert_eq!(r.json()["files"], 0);
    assert_eq!(r.json()["visible"], false);
}

// ---- DNS rebinding and cross-site requests ---------------------------------

#[test]
fn a_foreign_host_header_is_refused_even_with_the_token() {
    let f = fixture();
    for host in [
        "evil.example.com",
        "evil.example.com:47821",
        "127.0.0.1",
        "127.0.0.1.evil.com:80",
    ] {
        let r = Request::new("GET", "/v1/status")
            .bearer(TOKEN)
            .host(host)
            .send(f.port);
        assert_eq!(r.status, 403, "Host: {host}");
        let ping = Request::new("GET", "/v1/ping").host(host).send(f.port);
        assert_eq!(ping.status, 403, "even ping must refuse Host: {host}");
    }
}

#[test]
fn localhost_is_an_acceptable_host() {
    let f = fixture();
    let host = format!("localhost:{}", f.port);
    let r = Request::new("GET", "/v1/status")
        .bearer(TOKEN)
        .host(&host)
        .send(f.port);
    assert_eq!(r.status, 200);
}

#[test]
fn web_page_origins_are_refused_even_with_the_token() {
    let f = fixture();
    for origin in [
        "https://evil.example.com",
        "http://127.0.0.1:47821",
        "http://localhost",
        "null",
        "chrome-extension.evil.com",
    ] {
        let r = Request::new("GET", "/v1/notes")
            .bearer(TOKEN)
            .header("Origin", origin)
            .send(f.port);
        assert_eq!(r.status, 403, "Origin: {origin}");
        assert!(
            r.header("access-control-allow-origin").is_none(),
            "no CORS for {origin}"
        );
    }
    // A forged request that would create data must not.
    let r = Request::new("POST", "/v1/notes")
        .bearer(TOKEN)
        .header("Origin", "https://evil.example.com")
        .json(r#"{"text":"pwned"}"#)
        .send(f.port);
    assert_eq!(r.status, 403);
    assert!(f.store.notes().is_empty());
}

#[test]
fn extension_origins_are_allowed_and_get_cors_headers() {
    let f = fixture();
    for origin in [
        EXTENSION,
        "moz-extension://1234-abcd",
        "safari-web-extension://xyz",
    ] {
        let r = Request::new("GET", "/v1/status")
            .bearer(TOKEN)
            .header("Origin", origin)
            .send(f.port);
        assert_eq!(r.status, 200, "{origin}");
        assert_eq!(r.header("access-control-allow-origin"), Some(origin));
        assert_eq!(r.header("vary"), Some("Origin"));
    }
}

#[test]
fn preflight_for_an_extension_succeeds_without_a_token() {
    let f = fixture();
    let r = Request::new("OPTIONS", "/v1/files")
        .header("Origin", EXTENSION)
        .header("Access-Control-Request-Method", "POST")
        .header(
            "Access-Control-Request-Headers",
            "authorization, x-tsumiba-name, content-type",
        )
        .send(f.port);
    assert_eq!(r.status, 204);
    assert_eq!(r.header("access-control-allow-origin"), Some(EXTENSION));
    let allowed = r
        .header("access-control-allow-headers")
        .unwrap()
        .to_lowercase();
    for needed in [
        "authorization",
        "x-tsumiba-name",
        "x-tsumiba-source",
        "content-type",
    ] {
        assert!(allowed.contains(needed), "{needed} missing from {allowed}");
    }
    assert!(
        r.header("access-control-allow-methods")
            .unwrap()
            .contains("POST")
    );
}

#[test]
fn preflight_from_a_web_page_is_refused() {
    let f = fixture();
    let r = Request::new("OPTIONS", "/v1/notes")
        .header("Origin", "https://evil.example.com")
        .header("Access-Control-Request-Method", "POST")
        .send(f.port);
    assert_eq!(r.status, 403);
    assert!(r.header("access-control-allow-origin").is_none());
}

#[test]
fn responses_are_never_cached_or_sniffed() {
    let f = fixture();
    let r = f.get("/v1/status");
    assert_eq!(r.header("cache-control"), Some("no-store"));
    assert_eq!(r.header("x-content-type-options"), Some("nosniff"));
    assert!(
        r.header("content-type")
            .unwrap()
            .starts_with("application/json")
    );
}

// ---- notes ------------------------------------------------------------------

#[test]
fn notes_can_be_created_read_changed_and_deleted() {
    let f = fixture();

    let created = f.post_json("/v1/notes", r##"{"text":"# 買い物\n牛乳"}"##);
    assert_eq!(created.status, 201);
    let note = created.json();
    let id = note["id"].as_str().unwrap().to_string();
    assert_eq!(note["title"], "買い物");
    assert_eq!(note["body"], "# 買い物\n牛乳");
    assert_eq!(note["pinned"], false);

    let listed = f.get("/v1/notes").json();
    assert_eq!(listed.as_array().unwrap().len(), 1);

    let fetched = f.get(&format!("/v1/notes/{id}"));
    assert_eq!(fetched.status, 200);
    assert_eq!(fetched.json()["body"], "# 買い物\n牛乳");

    let patched = Request::new("PATCH", &format!("/v1/notes/{id}"))
        .bearer(TOKEN)
        .json(r#"{"text":"changed","pinned":true}"#)
        .send(f.port);
    assert_eq!(patched.status, 200);
    assert_eq!(patched.json()["body"], "changed");
    assert_eq!(patched.json()["pinned"], true);
    assert_eq!(f.store.note(&id).unwrap().body, "changed");

    // Pin alone leaves the text alone.
    let unpinned = Request::new("PATCH", &format!("/v1/notes/{id}"))
        .bearer(TOKEN)
        .json(r#"{"pinned":false}"#)
        .send(f.port);
    assert_eq!(unpinned.json()["body"], "changed");
    assert_eq!(unpinned.json()["pinned"], false);

    let deleted = Request::new("DELETE", &format!("/v1/notes/{id}"))
        .bearer(TOKEN)
        .send(f.port);
    assert_eq!(deleted.status, 204);
    assert!(deleted.body.is_empty());
    assert_eq!(f.get(&format!("/v1/notes/{id}")).status, 404);
    assert!(f.store.notes().is_empty());
}

#[test]
fn a_note_can_be_created_pinned() {
    let f = fixture();
    let r = f.post_json("/v1/notes", r#"{"text":"important","pinned":true}"#);
    assert_eq!(r.status, 201);
    assert_eq!(r.json()["pinned"], true);
}

#[test]
fn notes_can_be_searched_and_limited() {
    let f = fixture();
    for text in ["apple pie", "banana split", "Apple tart"] {
        f.post_json(
            "/v1/notes",
            &serde_json::json!({ "text": text }).to_string(),
        );
    }
    let found = f.get("/v1/notes?q=apple").json();
    assert_eq!(found.as_array().unwrap().len(), 2);
    let found = f.get("/v1/notes?q=APPLE+tart").json();
    assert_eq!(found.as_array().unwrap().len(), 1);
    let limited = f.get("/v1/notes?limit=1").json();
    assert_eq!(limited.as_array().unwrap().len(), 1);
    let none = f.get("/v1/notes?q=durian").json();
    assert!(none.as_array().unwrap().is_empty());
}

#[test]
fn bad_note_requests_get_clear_errors() {
    let f = fixture();
    assert_eq!(f.post_json("/v1/notes", "{ nope").status, 400);
    assert_eq!(f.post_json("/v1/notes", r#"{"wrong":"shape"}"#).status, 400);
    assert_eq!(f.get("/v1/notes/0123456789abcdef0123").status, 404);
    // Path traversal through the id.
    for id in ["..", "..%2F..%2Fetc%2Fpasswd", "not-an-id"] {
        let r = Request::new("PATCH", &format!("/v1/notes/{id}"))
            .bearer(TOKEN)
            .json(r#"{"text":"x"}"#)
            .send(f.port);
        assert!(matches!(r.status, 400 | 404), "{id}: {}", r.status);
    }
}

#[test]
fn a_huge_json_body_is_refused_before_it_is_read() {
    let f = fixture();
    let r = Request::new("POST", "/v1/notes")
        .bearer(TOKEN)
        .header("Content-Type", "application/json")
        .claimed_length(50 * 1024 * 1024)
        .send(f.port);
    assert_eq!(r.status, 413);
    assert!(f.store.notes().is_empty());
}

// ---- files ------------------------------------------------------------------

fn upload(
    f: &Fixture,
    name: &str,
    source: Option<&str>,
    content_type: &str,
    body: &[u8],
) -> common::Response {
    let mut request = Request::new("POST", "/v1/files")
        .bearer(TOKEN)
        .header("X-Tsumiba-Name", encode_header(name))
        .header("Content-Type", content_type)
        .body(body);
    if let Some(source) = source {
        request = request.header("X-Tsumiba-Source", encode_header(source));
    }
    request.send(f.port)
}

#[test]
fn a_file_can_be_uploaded_and_listed() {
    let f = fixture();
    let r = upload(
        &f,
        "スクリーンショット 1.png",
        Some("https://example.com/page?a=b&c=日本"),
        "image/png",
        b"\x89PNG fake bytes",
    );
    assert_eq!(r.status, 201, "{}", r.text());
    let file = r.json();
    assert_eq!(file["name"], "スクリーンショット 1.png");
    assert_eq!(file["size"], 15);
    assert_eq!(file["source"], "https://example.com/page?a=b&c=日本");
    assert_eq!(file["link"], false);

    let stored = std::fs::read(file["path"].as_str().unwrap()).unwrap();
    assert_eq!(stored, b"\x89PNG fake bytes");

    let listed = f.get("/v1/files").json();
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["id"], file["id"]);
}

#[test]
fn an_upload_without_an_extension_gets_one_from_the_content_type() {
    let f = fixture();
    let r = upload(&f, "image", None, "image/jpeg", b"jpeg");
    assert_eq!(r.json()["name"], "image.jpg");
    let r = upload(&f, "already.txt", None, "image/png", b"x");
    assert_eq!(
        r.json()["name"],
        "already.txt",
        "an existing extension is kept"
    );
    let r = upload(&f, "mystery", None, "application/x-unknown", b"x");
    assert_eq!(r.json()["name"], "mystery");
}

#[test]
fn hostile_file_names_cannot_escape_the_shelf() {
    let f = fixture();
    for (name, expected) in [
        ("../../../../etc/cron.d/evil", "evil"),
        ("..\\..\\windows\\system32\\evil.dll", "evil.dll"),
        ("CON", "_CON"),
        ("", "file"),
    ] {
        let r = upload(&f, name, None, "text/plain", b"x");
        assert_eq!(r.status, 201, "{name:?}");
        let file = r.json();
        assert!(
            file["name"].as_str().unwrap().starts_with(expected),
            "{name:?} became {}",
            file["name"]
        );
        let path = std::path::PathBuf::from(file["path"].as_str().unwrap());
        assert!(
            path.starts_with(f.store.root()),
            "{} escaped",
            path.display()
        );
    }
}

#[test]
fn uploads_over_the_limit_are_refused() {
    let f = fixture();
    f.config.write().unwrap().storage.max_upload_mb = 1;
    let exactly = vec![7u8; 1024 * 1024];
    assert_eq!(
        upload(&f, "ok.bin", None, "application/octet-stream", &exactly).status,
        201
    );

    let too_big = vec![7u8; 1024 * 1024 + 1];
    let r = upload(&f, "big.bin", None, "application/octet-stream", &too_big);
    assert_eq!(r.status, 413, "{}", r.text());
    assert_eq!(
        f.store.files().len(),
        1,
        "the refused upload left nothing behind"
    );
}

#[test]
fn a_chunked_upload_is_cut_off_at_the_limit() {
    // No Content-Length, so the early size check cannot help: only the
    // streaming limit stands between this and a full disk.
    let f = fixture();
    f.config.write().unwrap().storage.max_upload_mb = 1;
    let body = vec![9u8; 3 * 1024 * 1024];
    let r = Request::new("POST", "/v1/files")
        .bearer(TOKEN)
        .header("X-Tsumiba-Name", "stream.bin")
        .body(&body)
        .chunked()
        .send(f.port);
    assert_eq!(r.status, 413, "{}", r.text());
    assert!(f.store.files().is_empty(), "the partial file was removed");
    let leftovers = std::fs::read_dir(f.store.root().join("files"))
        .unwrap()
        .count();
    assert_eq!(leftovers, 0, "no stray directory either");

    // And a chunked upload within the limit works.
    let small = vec![1u8; 100 * 1024];
    let r = Request::new("POST", "/v1/files")
        .bearer(TOKEN)
        .header("X-Tsumiba-Name", "small.bin")
        .body(&small)
        .chunked()
        .send(f.port);
    assert_eq!(r.status, 201, "{}", r.text());
    assert_eq!(r.json()["size"], 100 * 1024);
}

#[test]
fn a_lying_content_length_cannot_sneak_past_the_limit() {
    let f = fixture();
    f.config.write().unwrap().storage.max_upload_mb = 1;
    // Claims to be tiny, sends 2 MiB. The server must stop at the limit, not at the claim.
    let body = vec![1u8; 2 * 1024 * 1024];
    let r = Request::new("POST", "/v1/files")
        .bearer(TOKEN)
        .header("X-Tsumiba-Name", "sneaky.bin")
        .body(&body)
        .claimed_length(10)
        .send(f.port);
    // tiny_http honours Content-Length, so only 10 bytes are read.
    assert!(r.status == 201 || r.status == 413, "{}", r.status);
    for file in f.store.files() {
        assert!(file.size <= 1024 * 1024, "{} bytes stored", file.size);
    }
}

#[test]
fn files_on_the_server_machine_can_be_imported() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("note.txt");
    std::fs::write(&real, "hello").unwrap();
    let real_str = real.to_str().unwrap();

    let body = serde_json::json!({
        "paths": [real_str, "relative/path.txt", dir.path().join("missing.txt").to_str().unwrap()]
    })
    .to_string();
    let r = f.post_json("/v1/files/import", &body);
    assert_eq!(r.status, 200);
    let result = r.json();
    assert_eq!(result["added"].as_array().unwrap().len(), 1);
    assert_eq!(result["added"][0]["name"], "note.txt");
    assert_eq!(result["added"][0]["link"], false);
    let failed = result["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 2);
    assert!(failed[0]["error"].as_str().unwrap().contains("absolute"));
}

#[test]
fn import_can_link_instead_of_copy() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("big.iso");
    std::fs::write(&real, "pretend this is large").unwrap();
    let body = serde_json::json!({ "paths": [real.to_str().unwrap()], "link": true }).to_string();
    let r = f.post_json("/v1/files/import", &body);
    assert_eq!(r.json()["added"][0]["link"], true);
    assert_eq!(f.store.files()[0].path, std::path::absolute(&real).unwrap());
}

#[test]
fn files_can_be_removed_without_touching_a_linked_original() {
    let f = fixture();
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("keep.txt");
    std::fs::write(&real, "precious").unwrap();
    let body = serde_json::json!({ "paths": [real.to_str().unwrap()], "link": true }).to_string();
    let id = f.post_json("/v1/files/import", &body).json()["added"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let r = Request::new("DELETE", &format!("/v1/files/{id}"))
        .bearer(TOKEN)
        .send(f.port);
    assert_eq!(r.status, 204);
    assert!(real.exists(), "the original must survive");
    assert!(f.store.files().is_empty());
    assert_eq!(
        Request::new("DELETE", &format!("/v1/files/{id}"))
            .bearer(TOKEN)
            .send(f.port)
            .status,
        404
    );
}

// ---- window control -----------------------------------------------------------

#[test]
fn window_actions_reach_the_window() {
    let f = fixture();
    for (action, expected) in [
        ("show", Command::Show),
        ("hide", Command::Hide),
        ("toggle", Command::Toggle),
    ] {
        let r = f.post_json("/v1/window", &format!(r#"{{"action":"{action}"}}"#));
        assert_eq!(r.status, 204, "{action}");
        assert_eq!(
            f.rx.recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            expected
        );
    }
    assert_eq!(
        f.post_json("/v1/window", r#"{"action":"explode"}"#).status,
        400
    );
    assert!(f.rx.try_recv().is_err(), "a bad action sent nothing");
}

#[test]
fn quit_reaches_the_window() {
    let f = fixture();
    assert_eq!(f.post_json("/v1/quit", "{}").status, 204);
    assert_eq!(
        f.rx.recv_timeout(std::time::Duration::from_secs(2))
            .unwrap(),
        Command::Quit
    );
}

// ---- routing --------------------------------------------------------------------

#[test]
fn unknown_routes_and_methods_are_404_json() {
    let f = fixture();
    for (method, path) in [
        ("GET", "/"),
        ("GET", "/v2/status"),
        ("GET", "/v1/nope"),
        ("PUT", "/v1/notes"),
        ("GET", "/v1/files/import"),
    ] {
        let r = Request::new(method, path).bearer(TOKEN).send(f.port);
        assert_eq!(r.status, 404, "{method} {path}");
        assert!(r.json()["error"].is_string());
    }
}

#[test]
fn concurrent_requests_all_succeed() {
    let f = fixture();
    let port = f.port;
    let handles: Vec<_> = (0..16)
        .map(|i| {
            std::thread::spawn(move || {
                Request::new("POST", "/v1/notes")
                    .bearer(TOKEN)
                    .json(&format!(r#"{{"text":"note {i}"}}"#))
                    .send(port)
                    .status
            })
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), 201);
    }
    assert_eq!(f.store.notes().len(), 16);
}
