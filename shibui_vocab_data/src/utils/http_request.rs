use std::fmt;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

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
    fn get(&self, url: &str) -> impl Future<Output = Result<String, FetchError>> + Send;
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

    pub async fn release_session(&self) {
        if let Some(keeper) = &self.flaresolverr {
            keeper.lock().await.release().await;
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

const PLAIN_USER_AGENT: &str = "Shibui-Dictionary-research/0.1 (personal study project)";
const CONTACT_URL_VARIABLE: &str = "CRAWLER_CONTACT_URL";
const MAX_RETRY_AFTER: Duration = Duration::from_secs(60);
const DEFAULT_RETRY_AFTER: Duration = Duration::from_secs(30);

pub fn user_agent_with(contact_url: Option<&str>) -> String {
    match contact_url.map(str::trim).filter(|url| !url.is_empty()) {
        Some(url) => format!("Shibui-Dictionary-research/0.1 ({url}) reqwest"),
        None => PLAIN_USER_AGENT.to_owned(),
    }
}

pub fn retry_after(header: Option<&str>) -> Duration {
    header
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(DEFAULT_RETRY_AFTER)
        .min(MAX_RETRY_AFTER)
}

#[derive(Clone)]
pub struct PlainHttpRequestMaker {
    client: reqwest::Client,
}

impl PlainHttpRequestMaker {
    pub fn new() -> Self {
        Self::with_contact(std::env::var(CONTACT_URL_VARIABLE).ok().as_deref())
    }

    pub fn with_contact(contact_url: Option<&str>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(user_agent_with(contact_url))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn send(&self, url: &str) -> Result<reqwest::Response, FetchError> {
        let response = self.client.get(url).send().await?;
        if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Ok(response);
        }

        let wait = retry_after(response.headers().get(reqwest::header::RETRY_AFTER).and_then(|value| value.to_str().ok()));
        println!("GET {url} -> status 429, waiting {} s as the server asked", wait.as_secs());
        tokio::time::sleep(wait).await;
        Ok(self.client.get(url).send().await?)
    }
}

impl HttpRequestMaker for PlainHttpRequestMaker {
    async fn get(&self, url: &str) -> Result<String, FetchError> {
        let response = self.send(url).await?;
        let status = response.status();
        println!("GET {url} -> status {status}");
        match status {
            status if status.is_success() => Ok(response.text().await?),
            reqwest::StatusCode::NOT_FOUND => Err(FetchError::NotFound(format!("{url} answered 404"))),
            status => Err(FetchError::Other(anyhow::anyhow!("{url} answered {status}"))),
        }
    }
}

#[cfg(test)]
mod plain_request_tests {
    use super::*;

    #[test]
    fn the_user_agent_carries_the_contact_url_when_one_is_set() {
        assert_eq!(
            user_agent_with(Some(" https://example.org/about ")),
            "Shibui-Dictionary-research/0.1 (https://example.org/about) reqwest"
        );
        assert_eq!(user_agent_with(None), PLAIN_USER_AGENT);
        assert_eq!(user_agent_with(Some("  ")), PLAIN_USER_AGENT);
    }

    #[test]
    fn retry_after_follows_the_server_but_never_waits_more_than_a_minute() {
        assert_eq!(retry_after(Some("41")), Duration::from_secs(41));
        assert_eq!(retry_after(Some("900")), Duration::from_secs(60));
        assert_eq!(retry_after(Some("soon")), Duration::from_secs(30));
        assert_eq!(retry_after(None), Duration::from_secs(30));
    }
}

#[derive(Clone)]
pub struct ChallengeHttpRequestMaker {
    keeper: Option<SharedKeeper>,
}

impl ChallengeHttpRequestMaker {
    pub fn new(warm_up_url: &str) -> Self {
        let keeper = std::env::var("FLARESOLVERR_URL").ok().map(|url| {
            Arc::new(Mutex::new(SessionKeeper::new(FlareSolverrClient::new(&url), warm_up_url)))
        });
        Self { keeper }
    }

    pub async fn start_session(&self) {
        if let Some(keeper) = &self.keeper {
            keeper.lock().await.start().await;
        }
    }

    pub async fn recycle_session(&self) {
        if let Some(keeper) = &self.keeper {
            keeper.lock().await.recycle().await;
        }
    }

    pub async fn release_session(&self) {
        if let Some(keeper) = &self.keeper {
            keeper.lock().await.release().await;
        }
    }
}

impl HttpRequestMaker for ChallengeHttpRequestMaker {
    async fn get(&self, url: &str) -> Result<String, FetchError> {
        let keeper = self
            .keeper
            .as_ref()
            .ok_or_else(|| FetchError::Other(anyhow::anyhow!("FLARESOLVERR_URL is not set")))?;
        let solution = keeper.lock().await.get(url).await?;
        println!("GET {url} (via FlareSolverr) -> status {} final_url {}", solution.status, solution.url);
        if solution.status == 404 {
            return Err(FetchError::NotFound(format!("Endpoint {url} answered 404")));
        }
        if solution.response.contains("Just a moment...") {
            return Err(FetchError::Other(anyhow::anyhow!("Endpoint {url} returned a Cloudflare challenge page")));
        }
        Ok(solution.response)
    }
}
