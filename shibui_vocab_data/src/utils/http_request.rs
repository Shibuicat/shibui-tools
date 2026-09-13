use std::fmt;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;


#[derive(Debug)]
pub enum FetchError {
    NotFound(String),
    Other(anyhow::Error),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::NotFound(msg) => write!(f, "{msg}"),
            FetchError::Other(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for FetchError {}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> Self {
        FetchError::Other(err.into())
    }
}

impl From<std::io::Error> for FetchError {
    fn from(err: std::io::Error) -> Self {
        FetchError::Other(err.into())
    }
}

pub trait HttpRequestMaker {
    fn get(&self, url: &str) -> impl Future<Output = Result<String, FetchError>>;
}

#[derive(serde::Serialize)]
struct FlareSolverrRequest<'a> {
    cmd: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(rename = "maxTimeout")]
    max_timeout: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<&'a str>,
}

#[derive(serde::Deserialize)]
struct FlareSolverrResponse {
    status: String,
    message: String,
    #[serde(default)]
    session: Option<String>,
    #[serde(default)]
    solution: Option<FlareSolverrSolution>,
}

#[derive(serde::Deserialize)]
struct FlareSolverrSolution {
    url: String,
    status: u16,
    response: String,
}

#[derive(Clone)]
pub struct DefaultHttpRequestMaker {
    client: reqwest::Client,
    // Set from the FLARESOLVERR_URL env var (e.g. http://localhost:8191). When
    // present, every GET routes through FlareSolverr's real-Chromium-backed
    // /v1 API instead of a direct reqwest call - a bare cf_clearance cookie
    // isn't enough on its own, since Cloudflare also binds it to the TLS
    // fingerprint of whatever client solved the challenge, which neither curl
    // nor reqwest can reproduce. Absent (e.g. the deployment binary, which has
    // no .env) falls back to the direct path unchanged.
    flaresolverr_url: Option<String>,
    // A FlareSolverr session keeps one browser context (and its solved
    // cf_clearance cookie) alive across requests, so only the first fetch
    // after startup pays the ~15-20s challenge-solving cost - every
    // subsequent one reuses it. Shared (not per-clone) since this struct is
    // held behind an Arc and cloned freely.
    flaresolverr_session: Arc<Mutex<Option<String>>>,
}

impl DefaultHttpRequestMaker {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            flaresolverr_url: std::env::var("FLARESOLVERR_URL").ok(),
            flaresolverr_session: Arc::new(Mutex::new(None)),
        }
    }

    fn check_not_found_or_challenge(url: &str, final_url: &str, html: &str) -> Result<(), FetchError> {
        let final_path = reqwest::Url::parse(final_url)
            .map(|u| u.path().to_string())
            .unwrap_or_default();
        if final_path == "/dictionary/english/" || final_path.starts_with("/spellcheck/") {
            return Err(FetchError::NotFound(format!(
                "Endpoint {url} doesn't exist (redirected to {final_url})"
            )));
        }
        if html.contains("Just a moment...") {
            return Err(FetchError::Other(anyhow::anyhow!(
                "Endpoint {url} returned a Cloudflare challenge page"
            )));
        }
        Ok(())
    }

    async fn create_flaresolverr_session(&self, flaresolverr_url: &str) -> Result<String, FetchError> {
        let request = FlareSolverrRequest {
            cmd: "sessions.create",
            url: None,
            max_timeout: 60_000,
            session: None,
        };
        let response: FlareSolverrResponse = self
            .client
            .post(format!("{flaresolverr_url}/v1"))
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if response.status != "ok" {
            return Err(FetchError::Other(anyhow::anyhow!(
                "FlareSolverr session create failed: {}",
                response.message
            )));
        }
        response.session.ok_or_else(|| {
            FetchError::Other(anyhow::anyhow!(
                "FlareSolverr sessions.create returned no session id"
            ))
        })
    }

    async fn ensure_flaresolverr_session(&self, flaresolverr_url: &str) -> Result<String, FetchError> {
        let mut current = self.flaresolverr_session.lock().await;
        if let Some(id) = current.as_ref() {
            return Ok(id.clone());
        }
        let id = self.create_flaresolverr_session(flaresolverr_url).await?;
        *current = Some(id.clone());
        Ok(id)
    }

    async fn request_solution(
        &self,
        flaresolverr_url: &str,
        url: &str,
        session: &str,
    ) -> Result<FlareSolverrSolution, FetchError> {
        let request = FlareSolverrRequest {
            cmd: "request.get",
            url: Some(url),
            max_timeout: 60_000,
            session: Some(session),
        };
        let response: FlareSolverrResponse = self
            .client
            .post(format!("{flaresolverr_url}/v1"))
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if response.status != "ok" {
            return Err(FetchError::Other(anyhow::anyhow!(
                "FlareSolverr failed for {url}: {}",
                response.message
            )));
        }
        response.solution.ok_or_else(|| {
            FetchError::Other(anyhow::anyhow!("FlareSolverr returned no solution for {url}"))
        })
    }

    async fn get_via_flaresolverr(&self, flaresolverr_url: &str, url: &str) -> Result<String, FetchError> {
        let session = self.ensure_flaresolverr_session(flaresolverr_url).await?;
        let solution = match self.request_solution(flaresolverr_url, url, &session).await {
            Ok(solution) => solution,
            Err(_) => {
                // The cached session may be stale (FlareSolverr restarted,
                // session expired) - drop it and retry once with a fresh one.
                *self.flaresolverr_session.lock().await = None;
                let fresh_session = self.ensure_flaresolverr_session(flaresolverr_url).await?;
                self.request_solution(flaresolverr_url, url, &fresh_session).await?
            }
        };

        println!(
            "GET {url} (via FlareSolverr) -> status {} final_url {}",
            solution.status, solution.url
        );

        Self::check_not_found_or_challenge(url, &solution.url, &solution.response)?;
        Ok(solution.response)
    }
}

impl HttpRequestMaker for DefaultHttpRequestMaker {
    async fn get(&self, url: &str) -> Result<String, FetchError> {
        if let Some(flaresolverr_url) = &self.flaresolverr_url {
            return self.get_via_flaresolverr(flaresolverr_url, url).await;
        }

        let response = self.client.get(url).send().await?;
        println!(
            "GET {url} -> status {} final_url {}",
            response.status(),
            response.url()
        );
        let final_url = response.url().to_string();
        let result = response.text().await?;
        Self::check_not_found_or_challenge(url, &final_url, &result)?;
        Ok(result)
    }
}

#[derive(Clone)]
pub struct FileBackedHttpRequestMaker<T: HttpRequestMaker + Clone> {
    inner: T,
    storage_dir: PathBuf,
}

impl<T: HttpRequestMaker + Clone> FileBackedHttpRequestMaker<T> {
    pub fn new(inner: T, storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            inner,
            storage_dir: storage_dir.into(),
        }
    }

    fn key_for_url(url: &str) -> String {
        url.rsplit('/').next().unwrap_or(url).to_lowercase()
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        self.storage_dir.join(format!("{key}.html"))
    }
}

impl<T: HttpRequestMaker + Clone> HttpRequestMaker for FileBackedHttpRequestMaker<T> {
    async fn get(&self, url: &str) -> Result<String, FetchError> {
        let path = self.path_for_key(&Self::key_for_url(url));

        if let Ok(html) = tokio::fs::read_to_string(&path).await {
            return Ok(html);
        }

        let html = self.inner.get(url).await?;

        if let Some(parent) = path.parent() {
            if let Err(err) = tokio::fs::create_dir_all(parent).await {
                eprintln!("Failed to create HTML storage dir {parent:?}: {err}");
                return Ok(html);
            }
        }
        if let Err(err) = tokio::fs::write(&path, &html).await {
            eprintln!("Failed to save HTML for {url} to {path:?}: {err}");
        }

        Ok(html)
    }
}
