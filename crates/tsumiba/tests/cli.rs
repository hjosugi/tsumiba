//! The command line, driven as a real process against a scratch home directory.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use common::Request;

fn tsumiba(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tsumiba"));
    command
        .env("TSUMIBA_HOME", home)
        .env_remove("TSUMIBA_DATA_DIR")
        .env_remove("TSUMIBA_CONFIG_DIR")
        // Nothing here should ever open a window, but make sure of it.
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY");
    command
}

struct Output {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn run(home: &Path, args: &[&str]) -> Output {
    let out = tsumiba(home).args(args).output().expect("run tsumiba");
    Output {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn run_with_stdin(home: &Path, args: &[&str], input: &str) -> Output {
    let mut child = tsumiba(home)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    Output {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

#[test]
fn version_and_help() {
    let home = tempfile::tempdir().unwrap();
    let v = run(home.path(), &["--version"]);
    assert!(v.ok);
    assert!(v.stdout.starts_with("tsumiba "), "{}", v.stdout);
    let h = run(home.path(), &["--help"]);
    assert!(h.ok);
    for word in [
        "note",
        "add",
        "list",
        "toggle",
        "serve",
        "https://hjosugi.github.io/tsumiba/",
    ] {
        assert!(
            h.stdout.contains(word),
            "--help should mention {word}:\n{}",
            h.stdout
        );
    }
}

#[test]
fn paths_point_into_the_home_directory() {
    let home = tempfile::tempdir().unwrap();
    let out = run(home.path(), &["paths"]);
    assert!(out.ok, "{}", out.stderr);
    for line in out.stdout.lines() {
        assert!(line.contains(home.path().to_str().unwrap()), "{line}");
    }
    assert!(out.stdout.contains("config.toml"));
}

#[test]
fn notes_added_without_an_app_land_in_the_store() {
    let home = tempfile::tempdir().unwrap();
    let id = run(home.path(), &["note", "buy", "milk"])
        .stdout
        .trim()
        .to_string();
    assert_eq!(id.len(), 20, "{id}");

    let listed = run(home.path(), &["list", "--notes"]);
    assert!(listed.ok);
    assert!(listed.stdout.contains(&id));
    assert!(listed.stdout.contains("buy milk"));
    assert!(listed.stdout.starts_with("note\t"), "{}", listed.stdout);

    let json: serde_json::Value =
        serde_json::from_str(&run(home.path(), &["list", "--json"]).stdout).unwrap();
    assert_eq!(json["notes"][0]["body"], "buy milk");
    assert_eq!(json["files"].as_array().unwrap().len(), 0);

    // On disk, as a plain Markdown file named after its id.
    let on_disk =
        std::fs::read_to_string(home.path().join("data/notes").join(format!("{id}.md"))).unwrap();
    assert_eq!(on_disk, "buy milk");
}

#[test]
fn notes_can_come_from_standard_input() {
    let home = tempfile::tempdir().unwrap();
    let out = run_with_stdin(home.path(), &["note"], "# 日本語\nstdin から\n");
    assert!(out.ok, "{}", out.stderr);
    let out = run_with_stdin(home.path(), &["note", "-", "--pin"], "pinned one");
    assert!(out.ok);
    let json: serde_json::Value =
        serde_json::from_str(&run(home.path(), &["list", "--json"]).stdout).unwrap();
    let notes = json["notes"].as_array().unwrap();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0]["pinned"], true, "pinned notes sort first");
    assert!(notes.iter().any(|n| n["title"] == "日本語"));
}

#[test]
fn an_empty_note_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let out = run_with_stdin(home.path(), &["note"], "  \n ");
    assert!(!out.ok);
    assert!(out.stderr.contains("nothing to save"), "{}", out.stderr);
    assert!(run(home.path(), &["list"]).stdout.is_empty());
}

#[test]
fn search_filters_notes() {
    let home = tempfile::tempdir().unwrap();
    for text in ["alpha one", "beta two", "alpha three"] {
        run(home.path(), &["note", text]);
    }
    let out = run(home.path(), &["list", "--notes", "-q", "alpha"]);
    assert_eq!(out.stdout.lines().count(), 2, "{}", out.stdout);
    assert!(!out.stdout.contains("beta"));
}

#[test]
fn files_are_copied_by_default_and_linked_on_request() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let a = work.path().join("a.txt");
    let b = work.path().join("b.txt");
    std::fs::write(&a, "alpha").unwrap();
    std::fs::write(&b, "beta").unwrap();

    let copied = run(home.path(), &["add", a.to_str().unwrap()]);
    assert!(copied.ok, "{}", copied.stderr);
    assert!(copied.stdout.contains("a.txt"));
    let linked = run(home.path(), &["add", "--link", b.to_str().unwrap()]);
    assert!(linked.ok);

    let json: serde_json::Value =
        serde_json::from_str(&run(home.path(), &["list", "--files", "--json"]).stdout).unwrap();
    let files = json["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    let by_name = |name: &str| files.iter().find(|f| f["name"] == name).unwrap();
    assert_eq!(by_name("a.txt")["link"], false);
    assert_eq!(by_name("b.txt")["link"], true);
    assert_eq!(
        std::fs::read_to_string(by_name("a.txt")["path"].as_str().unwrap()).unwrap(),
        "alpha"
    );
    // The copy is independent of the original.
    std::fs::remove_file(&a).unwrap();
    assert!(Path::new(by_name("a.txt")["path"].as_str().unwrap()).exists());
}

#[test]
fn relative_paths_work_and_missing_files_fail_loudly() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("here.txt"), "x").unwrap();

    let out = tsumiba(home.path())
        .current_dir(work.path())
        .args(["add", "here.txt", "missing.txt"])
        .output()
        .unwrap();
    assert!(
        !out.status.success(),
        "a missing file makes the command fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("missing.txt"), "{stderr}");
    // The good one was still added.
    assert!(
        run(home.path(), &["list", "--files"])
            .stdout
            .contains("here.txt")
    );
}

#[test]
fn adding_nothing_is_an_error() {
    let home = tempfile::tempdir().unwrap();
    assert!(!run(home.path(), &["add"]).ok);
}

#[test]
fn the_token_is_stable_and_can_be_replaced() {
    let home = tempfile::tempdir().unwrap();
    let first = run(home.path(), &["token"]).stdout.trim().to_string();
    let second = run(home.path(), &["token"]).stdout.trim().to_string();
    assert_eq!(first.len(), 64);
    assert!(first.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(first, second);

    let replaced = run(home.path(), &["token", "--regenerate"]);
    assert!(replaced.ok);
    assert_ne!(replaced.stdout.trim(), first);
    assert!(
        replaced.stderr.contains("Restart"),
        "the user is told what to do next"
    );
    assert_eq!(
        run(home.path(), &["token"]).stdout.trim(),
        replaced.stdout.trim()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(home.path().join("config/token"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "the token file is private");
    }
}

#[test]
fn hide_and_quit_say_so_when_nothing_is_running() {
    let home = tempfile::tempdir().unwrap();
    for command in ["hide", "quit"] {
        let out = run(home.path(), &[command]);
        assert!(!out.ok, "{command}");
        assert!(out.stderr.contains("not running"), "{}", out.stderr);
    }
}

#[test]
fn a_broken_config_is_reported_but_does_not_block_the_command_line() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("config")).unwrap();
    std::fs::write(home.path().join("config/config.toml"), "[panel\nbroken").unwrap();
    let out = run(home.path(), &["note", "still works"]);
    assert!(out.ok, "{}", out.stderr);
    assert!(out.stderr.contains("config.toml"), "{}", out.stderr);
}

// ---- with a running server -----------------------------------------------------

struct Serve {
    child: Child,
    port: u16,
}

impl Serve {
    fn start(home: &Path) -> Serve {
        let mut child = tsumiba(home)
            .args(["serve", "--port", "0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.as_mut().unwrap())
            .read_line(&mut line)
            .unwrap();
        let port: u16 = line
            .trim()
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(|| panic!("unexpected first line {line:?}"));
        Serve { child, port }
    }

    fn wait_for_exit(&mut self) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if let Ok(Some(status)) = self.child.try_wait() {
                return status.success();
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }
}

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn commands_go_through_the_running_app_and_quit_stops_it() {
    let home = tempfile::tempdir().unwrap();
    let mut server = Serve::start(home.path());
    let token = std::fs::read_to_string(home.path().join("config/token")).unwrap();
    let token = token.trim();

    // The CLI finds the server through its runtime record and talks to it.
    let id = run(home.path(), &["note", "via", "the", "api"])
        .stdout
        .trim()
        .to_string();
    assert_eq!(id.len(), 20);

    // Prove that it really went through the server: ask the server directly.
    let r = Request::new("GET", "/v1/notes")
        .bearer(token)
        .send(server.port);
    assert_eq!(r.status, 200);
    assert_eq!(r.json()[0]["body"], "via the api");
    assert_eq!(r.json()[0]["id"], id);

    // And `list` (also through the server) agrees.
    assert!(run(home.path(), &["list"]).stdout.contains("via the api"));

    // Adding files goes through the server too, and it does the copying.
    let work = tempfile::tempdir().unwrap();
    let file = work.path().join("shared.txt");
    std::fs::write(&file, "shared").unwrap();
    let added = run(home.path(), &["add", file.to_str().unwrap()]);
    assert!(added.ok, "{}", added.stderr);
    let files = Request::new("GET", "/v1/files")
        .bearer(token)
        .send(server.port)
        .json();
    assert_eq!(files[0]["name"], "shared.txt");

    // `serve` refuses to start twice.
    let second = run(home.path(), &["serve", "--port", "0"]);
    assert!(!second.ok);
    assert!(
        second.stderr.contains("already running"),
        "{}",
        second.stderr
    );

    // Window commands are accepted (there is no window to show, but the call succeeds).
    assert!(run(home.path(), &["hide"]).ok);

    // Quit stops the server and removes its record.
    assert!(run(home.path(), &["quit"]).ok);
    assert!(server.wait_for_exit(), "serve exited cleanly");
    assert!(!home.path().join("config/server.json").exists());
}

#[test]
fn a_stale_runtime_record_is_ignored() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("config")).unwrap();
    // A record from a crashed instance, pointing at a port nobody listens on.
    std::fs::write(
        home.path().join("config/server.json"),
        r#"{"port":1,"pid":1,"version":"0.0.0"}"#,
    )
    .unwrap();
    let out = run(home.path(), &["note", "offline"]);
    assert!(out.ok, "{}", out.stderr);
    assert!(run(home.path(), &["list"]).stdout.contains("offline"));
}

#[test]
fn another_users_instance_on_the_same_port_is_not_mistaken_for_ours() {
    // Two independent homes (think: two users, or two data directories). The
    // first runs a server. The second is configured with the *same port* but has
    // its own token, so the first server answers `ping` yet refuses its token.
    let first = tempfile::tempdir().unwrap();
    let mut server = Serve::start(first.path());

    let second = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(second.path().join("config")).unwrap();
    std::fs::write(
        second.path().join("config/config.toml"),
        format!("[api]\nport = {}\n", server.port),
    )
    .unwrap();

    let out = run(second.path(), &["note", "mine, not theirs"]);
    assert!(out.ok, "{}", out.stderr);

    // It went to the second home's own store...
    assert!(run(second.path(), &["list"]).stdout.contains("mine, not theirs"));
    // ...and the first server never saw it.
    let token = std::fs::read_to_string(first.path().join("config/token")).unwrap();
    let theirs = Request::new("GET", "/v1/notes").bearer(token.trim()).send(server.port);
    assert_eq!(theirs.json().as_array().unwrap().len(), 0);

    // A command that needs *our* app says it is not running, rather than failing
    // with a confusing "wrong token".
    let hide = run(second.path(), &["hide"]);
    assert!(!hide.ok);
    assert!(hide.stderr.contains("not running"), "{}", hide.stderr);

    assert!(run(first.path(), &["quit"]).ok);
    assert!(server.wait_for_exit());
}
