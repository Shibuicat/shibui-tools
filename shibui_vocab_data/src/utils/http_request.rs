use std::fmt;
use std::future::Future;
use std::sync::Arc;

use tokio::sync::Mutex;

use super::flaresolverr::FlareSolverrClient;
use super::session_keeper::SessionKeeper;

const WARM_UP_URL: &str = "https://dictionary.cambridge.org/";

type SharedKeeper = Arc<Mutex<SessionKeeper<FlareSolverrClient>>>;

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

#[derive(Clone)]
pub struct DefaultHttpRequestMaker {
    client: reqwest::Client,
    flaresolverr: Option<SharedKeeper>,
}

impl DefaultHttpRequestMaker {
    pub fn new() -> Self {
        let flaresolverr = std::env::var("FLARESOLVERR_URL").ok().map(|url| {
            Arc::new(Mutex::new(SessionKeeper::new(
                FlareSolverrClient::new(&url),
                WARM_UP_URL,
            )))
        });
        Self {
            client: reqwest::Client::new(),
            flaresolverr,
        }
    }

    pub async fn start_session(&self) {
        if let Some(keeper) = &self.flaresolverr {
            keeper.lock().await.start().await;
        }
    }

    pub async fn recycle_session(&self) {
        if let Some(keeper) = &self.flaresolverr {
            keeper.lock().await.recycle().await;
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

    async fn get_via_flaresolverr(&self, keeper: &SharedKeeper, url: &str) -> Result<String, FetchError> {
        let solution = keeper.lock().await.get(url).await?;
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
        if let Some(keeper) = &self.flaresolverr {
            return self.get_via_flaresolverr(keeper, url).await;
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
