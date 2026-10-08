use super::flaresolverr::{SessionBackend, Solution};
use super::http_request::FetchError;

pub struct SessionKeeper<B> {
    backend: B,
    warm_up_url: String,
    session: Option<String>,
}

impl<B: SessionBackend> SessionKeeper<B> {
    pub fn new(backend: B, warm_up_url: impl Into<String>) -> Self {
        Self {
            backend,
            warm_up_url: warm_up_url.into(),
            session: None,
        }
    }

    pub async fn start(&mut self) {
        self.session = None;
        self.destroy_all_sessions().await;
        self.open_warm_session().await;
    }

    pub async fn recycle(&mut self) {
        if let Some(old) = self.session.take() {
            self.backend.destroy_session(&old).await;
            self.open_warm_session().await;
        }
    }

    pub async fn release(&mut self) {
        if let Some(old) = self.session.take() {
            self.backend.destroy_session(&old).await;
        }
    }

    pub async fn get(&mut self, url: &str) -> Result<Solution, FetchError> {
        let session = self.session_or_new().await?;
        match self.backend.get(url, &session).await {
            Ok(solution) => Ok(solution),
            Err(_) => {
                self.backend.destroy_session(&session).await;
                self.session = None;
                let fresh = self.session_or_new().await?;
                self.backend.get(url, &fresh).await
            }
        }
    }

    async fn session_or_new(&mut self) -> Result<String, FetchError> {
        if let Some(session) = &self.session {
            return Ok(session.clone());
        }
        let session = self.backend.create_session().await?;
        self.session = Some(session.clone());
        Ok(session)
    }

    async fn destroy_all_sessions(&self) {
        for session in self.backend.list_sessions().await {
            self.backend.destroy_session(&session).await;
        }
    }

    async fn open_warm_session(&mut self) {
        let session = match self.session_or_new().await {
            Ok(session) => session,
            Err(err) => {
                eprintln!("Could not open a FlareSolverr session: {err}");
                return;
            }
        };
        match self.backend.get(&self.warm_up_url, &session).await {
            Ok(solution) => println!("Warmed FlareSolverr session {session} (status {})", solution.status),
            Err(err) => eprintln!("Could not warm FlareSolverr session {session}: {err}"),
        }
    }
}
