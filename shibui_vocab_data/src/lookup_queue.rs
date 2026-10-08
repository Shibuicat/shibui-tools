use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep_until, timeout, Instant};

use crate::scraper::WordDefinition;

pub const DEFAULT_QUEUE_CAPACITY: usize = 20;
pub const DEFAULT_LOOKUP_TIMEOUT_SECS: u64 = 90;
pub const DEFAULT_SESSION_MAX_AGE_SECS: u64 = 86_400;
pub const DEFAULT_SESSION_IDLE_SECS: u64 = 600;

#[derive(Debug)]
pub enum LookupError {
    Busy,
    TimedOut,
    Unavailable,
    Failed(anyhow::Error),
}

impl fmt::Display for LookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LookupError::Busy => write!(f, "The lookup queue is full, try again later"),
            LookupError::TimedOut => write!(f, "The lookup took too long"),
            LookupError::Unavailable => write!(f, "The lookup worker is not running"),
            LookupError::Failed(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for LookupError {}

pub type LookupFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Option<WordDefinition>, LookupError>> + Send + 'a>>;

pub trait WordLookup: Send + Sync {
    fn lookup<'a>(&'a self, word: &'a str) -> LookupFuture<'a>;
}

pub trait LookupBackend: Send + Sync + 'static {
    fn fetch(&self, word: &str) -> impl Future<Output = anyhow::Result<Option<WordDefinition>>> + Send;
    fn rotate_session(&self) -> impl Future<Output = ()> + Send;
    fn release_session(&self) -> impl Future<Output = ()> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LookupSettings {
    pub capacity: usize,
    pub timeout: Duration,
    pub session_max_age: Duration,
    pub session_idle_timeout: Duration,
}

impl LookupSettings {
    pub fn from_env() -> Self {
        Self::from_values(
            std::env::var("LOOKUP_QUEUE_CAPACITY").ok().as_deref(),
            std::env::var("LOOKUP_TIMEOUT_SECS").ok().as_deref(),
            std::env::var("FLARESOLVERR_SESSION_MAX_AGE_SECS").ok().as_deref(),
            std::env::var("FLARESOLVERR_SESSION_IDLE_SECS").ok().as_deref(),
        )
    }

    pub fn from_values(
        capacity: Option<&str>,
        timeout_secs: Option<&str>,
        max_age_secs: Option<&str>,
        idle_secs: Option<&str>,
    ) -> Self {
        Self {
            capacity: positive_or(capacity, DEFAULT_QUEUE_CAPACITY as u64) as usize,
            timeout: Duration::from_secs(positive_or(timeout_secs, DEFAULT_LOOKUP_TIMEOUT_SECS)),
            session_max_age: Duration::from_secs(positive_or(max_age_secs, DEFAULT_SESSION_MAX_AGE_SECS)),
            session_idle_timeout: Duration::from_secs(positive_or(idle_secs, DEFAULT_SESSION_IDLE_SECS)),
        }
    }
}

pub fn positive_or(value: Option<&str>, default: u64) -> u64 {
    value
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|number| *number >= 1)
        .unwrap_or(default)
}

struct Job {
    word: String,
    reply: oneshot::Sender<anyhow::Result<Option<WordDefinition>>>,
}

pub struct QueuedWordLookup {
    jobs: mpsc::Sender<Job>,
    timeout: Duration,
}

impl QueuedWordLookup {
    pub fn start<B: LookupBackend>(backend: Arc<B>, settings: LookupSettings) -> Self {
        let (jobs, inbox) = mpsc::channel(settings.capacity);
        tokio::spawn(run_worker(backend, inbox, settings));
        Self {
            jobs,
            timeout: settings.timeout,
        }
    }

    fn enqueue(&self, word: &str) -> Result<oneshot::Receiver<anyhow::Result<Option<WordDefinition>>>, LookupError> {
        let (reply, response) = oneshot::channel();
        let job = Job {
            word: word.to_string(),
            reply,
        };
        self.jobs.try_send(job).map_err(|err| match err {
            mpsc::error::TrySendError::Full(_) => LookupError::Busy,
            mpsc::error::TrySendError::Closed(_) => LookupError::Unavailable,
        })?;
        Ok(response)
    }
}

impl WordLookup for QueuedWordLookup {
    fn lookup<'a>(&'a self, word: &'a str) -> LookupFuture<'a> {
        Box::pin(async move {
            let response = self.enqueue(word)?;
            match timeout(self.timeout, response).await {
                Ok(Ok(result)) => result.map_err(LookupError::Failed),
                Ok(Err(_)) => Err(LookupError::Unavailable),
                Err(_) => Err(LookupError::TimedOut),
            }
        })
    }
}

async fn run_worker<B: LookupBackend>(backend: Arc<B>, mut inbox: mpsc::Receiver<Job>, settings: LookupSettings) {
    let mut next_rotation = Instant::now() + settings.session_max_age;
    let mut release_at = Some(Instant::now() + settings.session_idle_timeout);
    loop {
        tokio::select! {
            job = inbox.recv() => match job {
                Some(job) => {
                    serve(backend.as_ref(), job).await;
                    release_at = Some(Instant::now() + settings.session_idle_timeout);
                }
                None => break,
            },
            _ = sleep_until(next_rotation) => {
                backend.rotate_session().await;
                next_rotation = Instant::now() + settings.session_max_age;
            }
            _ = sleep_until(release_at.unwrap_or(next_rotation)), if release_at.is_some() => {
                backend.release_session().await;
                release_at = None;
            }
        }
    }
}

async fn serve<B: LookupBackend>(backend: &B, job: Job) {
    if job.reply.is_closed() {
        return;
    }
    let result = backend.fetch(&job.word).await;
    let _ = job.reply.send(result);
}
