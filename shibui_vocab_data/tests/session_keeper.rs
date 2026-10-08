use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use shibui_vocab_data::utils::flaresolverr::{SessionBackend, Solution};
use shibui_vocab_data::utils::http_request::FetchError;
use shibui_vocab_data::utils::session_keeper::SessionKeeper;

const WARM_UP: &str = "https://warm.up/";

#[derive(Clone, Default)]
struct FakeBackend {
    calls: Arc<Mutex<Vec<String>>>,
    existing: Arc<Mutex<Vec<String>>>,
    next_id: Arc<AtomicUsize>,
    failing_gets: Arc<AtomicUsize>,
    failing_creates: Arc<AtomicUsize>,
}

impl FakeBackend {
    fn record(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn consume(counter: &AtomicUsize) -> bool {
        counter
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| left.checked_sub(1))
            .is_ok()
    }
}

impl SessionBackend for FakeBackend {
    async fn create_session(&self) -> Result<String, FetchError> {
        if Self::consume(&self.failing_creates) {
            self.record("create-failed".to_string());
            return Err(FetchError::Other(anyhow::anyhow!("create failed")));
        }
        let id = format!("s{}", self.next_id.fetch_add(1, Ordering::SeqCst) + 1);
        self.record(format!("create {id}"));
        Ok(id)
    }

    async fn destroy_session(&self, session: &str) {
        self.record(format!("destroy {session}"));
    }

    async fn list_sessions(&self) -> Vec<String> {
        self.existing.lock().unwrap().clone()
    }

    async fn get(&self, url: &str, session: &str) -> Result<Solution, FetchError> {
        if Self::consume(&self.failing_gets) {
            self.record(format!("get-failed {session} {url}"));
            return Err(FetchError::Other(anyhow::anyhow!("get failed")));
        }
        self.record(format!("get {session} {url}"));
        Ok(Solution {
            url: url.to_string(),
            status: 200,
            response: "<html></html>".to_string(),
        })
    }
}

fn keeper(backend: &FakeBackend) -> SessionKeeper<FakeBackend> {
    SessionKeeper::new(backend.clone(), WARM_UP)
}

#[tokio::test]
async fn start_removes_leftover_sessions_then_opens_and_warms_one() {
    let backend = FakeBackend::default();
    *backend.existing.lock().unwrap() = vec!["old1".to_string(), "old2".to_string()];
    let mut keeper = keeper(&backend);

    keeper.start().await;

    assert_eq!(
        backend.calls(),
        vec!["destroy old1", "destroy old2", "create s1", "get s1 https://warm.up/"]
    );
}

#[tokio::test]
async fn requests_reuse_the_warmed_session() {
    let backend = FakeBackend::default();
    let mut keeper = keeper(&backend);
    keeper.start().await;

    keeper.get("https://a/").await.unwrap();
    keeper.get("https://b/").await.unwrap();

    let gets: Vec<_> = backend.calls().into_iter().filter(|c| c.starts_with("get s")).collect();
    assert_eq!(gets, vec!["get s1 https://warm.up/", "get s1 https://a/", "get s1 https://b/"]);
    assert_eq!(backend.next_id.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_failed_request_replaces_the_session_and_retries_once() {
    let backend = FakeBackend::default();
    let mut keeper = keeper(&backend);
    keeper.start().await;
    backend.failing_gets.store(1, Ordering::SeqCst);

    let solution = keeper.get("https://a/").await.unwrap();

    assert_eq!(solution.url, "https://a/");
    let calls = backend.calls();
    let tail = &calls[calls.len() - 4..];
    assert_eq!(
        tail,
        ["get-failed s1 https://a/", "destroy s1", "create s2", "get s2 https://a/"]
    );
}

#[tokio::test]
async fn a_request_that_fails_twice_returns_the_error() {
    let backend = FakeBackend::default();
    let mut keeper = keeper(&backend);
    keeper.start().await;
    backend.failing_gets.store(2, Ordering::SeqCst);

    assert!(keeper.get("https://a/").await.is_err());
}

#[tokio::test]
async fn recycle_destroys_the_old_session_before_creating_the_new_one() {
    let backend = FakeBackend::default();
    let mut keeper = keeper(&backend);
    keeper.start().await;

    keeper.recycle().await;

    let calls = backend.calls();
    assert_eq!(
        calls[calls.len() - 3..],
        ["destroy s1", "create s2", "get s2 https://warm.up/"]
    );
}

#[tokio::test]
async fn failed_warm_up_keeps_the_session_for_the_next_request() {
    let backend = FakeBackend::default();
    backend.failing_gets.store(1, Ordering::SeqCst);
    let mut keeper = keeper(&backend);

    keeper.start().await;
    keeper.get("https://a/").await.unwrap();

    assert!(backend.calls().contains(&"get s1 https://a/".to_string()));
    assert_eq!(backend.next_id.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_failed_create_at_start_is_retried_on_the_first_request() {
    let backend = FakeBackend::default();
    backend.failing_creates.store(1, Ordering::SeqCst);
    let mut keeper = keeper(&backend);

    keeper.start().await;
    keeper.get("https://a/").await.unwrap();

    assert!(backend.calls().contains(&"get s1 https://a/".to_string()));
}
