use std::future::Future;

use super::http_request::FetchError;

const COMMAND_TIMEOUT_MS: u64 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Solution {
    pub url: String,
    pub status: u16,
    pub response: String,
}

pub trait SessionBackend {
    fn create_session(&self) -> impl Future<Output = Result<String, FetchError>> + Send;
    fn destroy_session(&self, session: &str) -> impl Future<Output = ()> + Send;
    fn list_sessions(&self) -> impl Future<Output = Vec<String>> + Send;
    fn get(&self, url: &str, session: &str) -> impl Future<Output = Result<Solution, FetchError>> + Send;
}

#[derive(serde::Serialize)]
struct Command<'a> {
    cmd: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(rename = "maxTimeout")]
    max_timeout: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct Reply {
    status: String,
    message: String,
    #[serde(default)]
    session: Option<String>,
    #[serde(default)]
    sessions: Vec<String>,
    #[serde(default)]
    solution: Option<RawSolution>,
}

#[derive(serde::Deserialize)]
struct RawSolution {
    url: String,
    status: u16,
    response: String,
}

#[derive(Clone)]
pub struct FlareSolverrClient {
    client: reqwest::Client,
    endpoint: String,
}

impl FlareSolverrClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: format!("{}/v1", base_url.trim_end_matches('/')),
        }
    }

    async fn send(&self, command: Command<'_>) -> Result<Reply, FetchError> {
        let reply: Reply = self
            .client
            .post(&self.endpoint)
            .json(&command)
            .send()
            .await?
            .json()
            .await?;
        if reply.status != "ok" {
            return Err(FetchError::Other(anyhow::anyhow!(
                "FlareSolverr {} failed: {}",
                command.cmd,
                reply.message
            )));
        }
        Ok(reply)
    }
}

impl SessionBackend for FlareSolverrClient {
    async fn create_session(&self) -> Result<String, FetchError> {
        let reply = self
            .send(Command {
                cmd: "sessions.create",
                url: None,
                max_timeout: COMMAND_TIMEOUT_MS,
                session: None,
            })
            .await?;
        reply.session.ok_or_else(|| {
            FetchError::Other(anyhow::anyhow!("FlareSolverr sessions.create returned no session id"))
        })
    }

    async fn destroy_session(&self, session: &str) {
        let outcome = self
            .send(Command {
                cmd: "sessions.destroy",
                url: None,
                max_timeout: COMMAND_TIMEOUT_MS,
                session: Some(session),
            })
            .await;
        if let Err(err) = outcome {
            eprintln!("Failed to destroy FlareSolverr session {session}: {err}");
        }
    }

    async fn list_sessions(&self) -> Vec<String> {
        let outcome = self
            .send(Command {
                cmd: "sessions.list",
                url: None,
                max_timeout: COMMAND_TIMEOUT_MS,
                session: None,
            })
            .await;
        match outcome {
            Ok(reply) => reply.sessions,
            Err(err) => {
                eprintln!("Failed to list FlareSolverr sessions: {err}");
                Vec::new()
            }
        }
    }

    async fn get(&self, url: &str, session: &str) -> Result<Solution, FetchError> {
        let reply = self
            .send(Command {
                cmd: "request.get",
                url: Some(url),
                max_timeout: COMMAND_TIMEOUT_MS,
                session: Some(session),
            })
            .await?;
        reply
            .solution
            .map(|raw| Solution {
                url: raw.url,
                status: raw.status,
                response: raw.response,
            })
            .ok_or_else(|| FetchError::Other(anyhow::anyhow!("FlareSolverr returned no solution for {url}")))
    }
}
