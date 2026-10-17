//! A client for the API of an already running tsumiba.
//!
//! The command line uses it so `tsumiba note ...` and `tsumiba add ...` go
//! through the running app (which keeps its window current) instead of writing
//! behind its back. When no app is running the CLI falls back to the store.

use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tsumiba_core::api::{
    DEFAULT_PORT, ErrorBody, FileDto, ImportRequest, ImportResult, NewNote, NoteDto, Ping, Status,
    WindowAction, WindowRequest,
};
use tsumiba_core::runtime::ServerInfo;
use tsumiba_core::{Config, Dirs, token};

#[derive(Debug)]
pub enum ClientError {
    /// Nothing is listening, or what listens is not tsumiba.
    NotRunning,
    /// The server answered with an error.
    Api { status: u16, message: String },
    /// The request could not be completed.
    Transport(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::NotRunning => write!(f, "tsumiba is not running"),
            ClientError::Api { status, message } => write!(f, "{message} (HTTP {status})"),
            ClientError::Transport(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ClientError {}

pub struct Client {
    base: String,
    token: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(port: u16, token: String) -> Client {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(30)))
            .timeout_connect(Some(Duration::from_millis(800)))
            // Handle 4xx/5xx ourselves so the message in the body is not lost.
            .http_status_as_error(false)
            .build()
            .into();
        Client {
            base: format!("http://127.0.0.1:{port}/v1"),
            token,
            agent,
        }
    }

    /// Find *this user's* running instance: first the port it recorded, then the
    /// configured one. `None` when nothing answers like tsumiba.
    ///
    /// Answering to `ping` is not enough. The port is shared by the whole
    /// machine, so the program that answers may be another user's tsumiba, or one
    /// using a different data directory. It is ours only if it accepts our token.
    pub fn discover(dirs: &Dirs, config: &Config) -> Option<Client> {
        let token = token::load_or_create(&dirs.token_file()).ok()?;
        let mut ports: Vec<u16> = Vec::new();
        if let Some(info) = ServerInfo::read(&dirs.runtime_file()) {
            ports.push(info.port);
        }
        let configured = if config.api.port == 0 {
            DEFAULT_PORT
        } else {
            config.api.port
        };
        if !ports.contains(&configured) {
            ports.push(configured);
        }
        ports.into_iter().find_map(|port| {
            let client = Client::new(port, token.clone());
            client.ping().ok().filter(Ping::is_tsumiba)?;
            client.status().ok().map(|_| client)
        })
    }

    pub fn ping(&self) -> Result<Ping, ClientError> {
        self.get("/ping")
    }

    pub fn status(&self) -> Result<Status, ClientError> {
        self.get("/status")
    }

    pub fn window(&self, action: WindowAction) -> Result<(), ClientError> {
        self.post_no_content("/window", &WindowRequest { action })
    }

    pub fn quit(&self) -> Result<(), ClientError> {
        self.post_no_content("/quit", &serde_json::json!({}))
    }

    pub fn notes(&self, query: &str) -> Result<Vec<NoteDto>, ClientError> {
        let q = tsumiba_core::api::encode_header(query);
        self.get(&format!("/notes?q={q}"))
    }

    pub fn create_note(&self, text: &str, pinned: bool) -> Result<NoteDto, ClientError> {
        self.post(
            "/notes",
            &NewNote {
                text: text.to_string(),
                pinned,
            },
        )
    }

    pub fn files(&self) -> Result<Vec<FileDto>, ClientError> {
        self.get("/files")
    }

    pub fn import(&self, paths: Vec<String>, link: bool) -> Result<ImportResult, ClientError> {
        self.post("/files/import", &ImportRequest { paths, link })
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let response = self
            .agent
            .get(format!("{}{path}", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .call()
            .map_err(transport)?;
        parse(response)
    }

    fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ClientError> {
        let response = self
            .agent
            .post(format!("{}{path}", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .send_json(body)
            .map_err(transport)?;
        parse(response)
    }

    fn post_no_content<B: Serialize>(&self, path: &str, body: &B) -> Result<(), ClientError> {
        let response = self
            .agent
            .post(format!("{}{path}", self.base))
            .header("Authorization", format!("Bearer {}", self.token))
            .send_json(body)
            .map_err(transport)?;
        check(response).map(|_| ())
    }
}

fn transport(e: ureq::Error) -> ClientError {
    match e {
        ureq::Error::ConnectionFailed | ureq::Error::Io(_) => ClientError::NotRunning,
        other => ClientError::Transport(other.to_string()),
    }
}

fn check(
    mut response: ureq::http::Response<ureq::Body>,
) -> Result<ureq::http::Response<ureq::Body>, ClientError> {
    let status = response.status().as_u16();
    if (200..300).contains(&status) {
        return Ok(response);
    }
    let message = response
        .body_mut()
        .read_json::<ErrorBody>()
        .map(|b| b.error)
        .unwrap_or_else(|_| format!("the server answered {status}"));
    Err(ClientError::Api { status, message })
}

fn parse<T: DeserializeOwned>(
    response: ureq::http::Response<ureq::Body>,
) -> Result<T, ClientError> {
    let mut response = check(response)?;
    response
        .body_mut()
        .read_json::<T>()
        .map_err(|e| ClientError::Transport(e.to_string()))
}
