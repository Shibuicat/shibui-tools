use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shibui_vocab_data::lookup_queue::{
    positive_or, LookupBackend, LookupError, LookupSettings, QueuedWordLookup, WordLookup,
    DEFAULT_LOOKUP_TIMEOUT_SECS, DEFAULT_QUEUE_CAPACITY, DEFAULT_SESSION_IDLE_SECS,
    DEFAULT_SESSION_MAX_AGE_SECS,
};
use shibui_vocab_data::scraper::WordDefinition;
use tokio::sync::Semaphore;

fn definition(word: &str) -> WordDefinition {
    WordDefinition {
        word: word.to_string(),
        classes: Vec::new(),
        extracted_html: None,
    }
}

struct FakeBackend {
    gate: Semaphore,
    delay: Duration,
    fetched: Mutex<Vec<String>>,
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    rotations: AtomicUsize,
    releases: AtomicUsize,
}

impl FakeBackend {
    fn open() -> Arc<Self> {
        Self::with(Semaphore::MAX_PERMITS, Duration::ZERO)
    }

    fn closed() -> Arc<Self> {
        Self::with(0, Duration::ZERO)
    }

    fn slow(delay: Duration) -> Arc<Self> {
        Self::with(Semaphore::MAX_PERMITS, delay)
    }

    fn with(permits: usize, delay: Duration) -> Arc<Self> {
        Arc::new(Self {
            gate: Semaphore::new(permits),
            delay,
            fetched: Mutex::new(Vec::new()),
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
            rotations: AtomicUsize::new(0),
            releases: AtomicUsize::new(0),
        })
    }

    fn fetched(&self) -> Vec<String> {
        self.fetched.lock().unwrap().clone()
    }
}

impl LookupBackend for FakeBackend {
    async fn fetch(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
        self.fetched.lock().unwrap().push(word.to_string());
        let running = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(running, Ordering::SeqCst);
        let _permit = self.gate.acquire().await.unwrap();
        tokio::time::sleep(self.delay).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        match word {
            "missing" => Ok(None),
            "broken" => Err(anyhow::anyhow!("boom")),
            _ => Ok(Some(definition(word))),
        }
    }

    async fn rotate_session(&self) {
        self.rotations.fetch_add(1, Ordering::SeqCst);
    }

    async fn release_session(&self) {
        self.releases.fetch_add(1, Ordering::SeqCst);
    }
}

const NEVER_IDLE_SECS: u64 = 10_000_000;

fn settings(capacity: usize, timeout_secs: u64, max_age_secs: u64) -> LookupSettings {
    settings_with_idle(capacity, timeout_secs, max_age_secs, NEVER_IDLE_SECS)
}

fn settings_with_idle(capacity: usize, timeout_secs: u64, max_age_secs: u64, idle_secs: u64) -> LookupSettings {
    LookupSettings {
        capacity,
        timeout: Duration::from_secs(timeout_secs),
        session_max_age: Duration::from_secs(max_age_secs),
        session_idle_timeout: Duration::from_secs(idle_secs),
    }
}

async fn let_tasks_run() {
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
}

#[test]
fn settings_default_when_unset() {
    let defaults = LookupSettings::from_values(None, None, None, None);

    assert_eq!(defaults.capacity, DEFAULT_QUEUE_CAPACITY);
    assert_eq!(defaults.timeout, Duration::from_secs(DEFAULT_LOOKUP_TIMEOUT_SECS));
    assert_eq!(defaults.session_max_age, Duration::from_secs(DEFAULT_SESSION_MAX_AGE_SECS));
    assert_eq!(defaults.session_idle_timeout, Duration::from_secs(DEFAULT_SESSION_IDLE_SECS));
}

#[test]
fn settings_read_valid_values() {
    let parsed = LookupSettings::from_values(Some("5"), Some(" 30 "), Some("3600"), Some("120"));

    assert_eq!(parsed, settings_with_idle(5, 30, 3600, 120));
}

#[test]
fn settings_fall_back_on_zero_negative_or_garbage() {
    assert_eq!(positive_or(Some("0"), 7), 7);
    assert_eq!(positive_or(Some("-3"), 7), 7);
    assert_eq!(positive_or(Some("many"), 7), 7);
    assert_eq!(positive_or(Some(""), 7), 7);
    assert_eq!(positive_or(Some("9"), 7), 9);
}

#[tokio::test]
async fn returns_the_definition_from_the_backend() {
    let lookup = QueuedWordLookup::start(FakeBackend::open(), settings(5, 90, 3600));

    let result = lookup.lookup("hello").await.unwrap();

    assert_eq!(result.unwrap().word, "hello");
}

#[tokio::test]
async fn passes_a_missing_word_through_as_none() {
    let lookup = QueuedWordLookup::start(FakeBackend::open(), settings(5, 90, 3600));

    assert!(lookup.lookup("missing").await.unwrap().is_none());
}

#[tokio::test]
async fn reports_a_backend_error_as_failed() {
    let lookup = QueuedWordLookup::start(FakeBackend::open(), settings(5, 90, 3600));

    let error = lookup.lookup("broken").await.unwrap_err();

    assert!(matches!(error, LookupError::Failed(_)));
}

#[tokio::test(start_paused = true)]
async fn serves_one_lookup_at_a_time() {
    let backend = FakeBackend::slow(Duration::from_millis(10));
    let lookup = Arc::new(QueuedWordLookup::start(backend.clone(), settings(10, 90, 3600)));

    let handles: Vec<_> = (0..5)
        .map(|index| {
            let lookup = Arc::clone(&lookup);
            tokio::spawn(async move { lookup.lookup(&format!("word{index}")).await })
        })
        .collect();
    for handle in handles {
        assert!(handle.await.unwrap().unwrap().is_some());
    }

    assert_eq!(backend.max_in_flight.load(Ordering::SeqCst), 1);
    assert_eq!(backend.fetched().len(), 5);
}

#[tokio::test(start_paused = true)]
async fn serves_lookups_in_the_order_they_arrived() {
    let backend = FakeBackend::slow(Duration::from_millis(10));
    let lookup = Arc::new(QueuedWordLookup::start(backend.clone(), settings(10, 90, 3600)));

    let mut handles = Vec::new();
    for word in ["a", "b", "c"] {
        let lookup = Arc::clone(&lookup);
        handles.push(tokio::spawn(async move { lookup.lookup(word).await }));
        let_tasks_run().await;
    }
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    assert_eq!(backend.fetched(), vec!["a", "b", "c"]);
}

#[tokio::test(start_paused = true)]
async fn rejects_with_busy_when_the_queue_is_full() {
    let backend = FakeBackend::closed();
    let lookup = Arc::new(QueuedWordLookup::start(backend.clone(), settings(1, 90, 3600)));
    let running = tokio::spawn({
        let lookup = Arc::clone(&lookup);
        async move { lookup.lookup("first").await }
    });
    let_tasks_run().await;
    let waiting = tokio::spawn({
        let lookup = Arc::clone(&lookup);
        async move { lookup.lookup("second").await }
    });
    let_tasks_run().await;

    let error = lookup.lookup("third").await.unwrap_err();

    assert!(matches!(error, LookupError::Busy));
    backend.gate.add_permits(10);
    assert!(running.await.unwrap().is_ok());
    assert!(waiting.await.unwrap().is_ok());
}

#[tokio::test(start_paused = true)]
async fn times_out_when_the_backend_is_too_slow() {
    let backend = FakeBackend::slow(Duration::from_secs(500));
    let lookup = QueuedWordLookup::start(backend, settings(5, 1, 3600));

    let error = lookup.lookup("slow").await.unwrap_err();

    assert!(matches!(error, LookupError::TimedOut));
}

#[tokio::test(start_paused = true)]
async fn skips_a_queued_lookup_whose_caller_gave_up() {
    let backend = FakeBackend::closed();
    let lookup = Arc::new(QueuedWordLookup::start(backend.clone(), settings(5, 1, 3600)));
    let first = tokio::spawn({
        let lookup = Arc::clone(&lookup);
        async move { lookup.lookup("first").await }
    });
    let_tasks_run().await;
    let second = tokio::spawn({
        let lookup = Arc::clone(&lookup);
        async move { lookup.lookup("second").await }
    });
    let_tasks_run().await;

    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(matches!(second.await.unwrap(), Err(LookupError::TimedOut)));
    assert!(matches!(first.await.unwrap(), Err(LookupError::TimedOut)));
    backend.gate.add_permits(10);
    let_tasks_run().await;

    assert_eq!(backend.fetched(), vec!["first"]);
}

#[tokio::test(start_paused = true)]
async fn rotates_the_session_on_schedule_while_idle() {
    let backend = FakeBackend::open();
    let _lookup = QueuedWordLookup::start(backend.clone(), settings(5, 90, 10));
    let_tasks_run().await;

    for _ in 0..2 {
        tokio::time::advance(Duration::from_secs(11)).await;
        let_tasks_run().await;
    }

    assert_eq!(backend.rotations.load(Ordering::SeqCst), 2);
}

#[tokio::test(start_paused = true)]
async fn keeps_serving_after_a_rotation() {
    let backend = FakeBackend::open();
    let lookup = QueuedWordLookup::start(backend.clone(), settings(5, 90, 10));
    let_tasks_run().await;
    tokio::time::advance(Duration::from_secs(11)).await;
    let_tasks_run().await;

    let result = lookup.lookup("after").await.unwrap();

    assert_eq!(result.unwrap().word, "after");
    assert_eq!(backend.rotations.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn releases_the_session_once_after_it_has_been_idle() {
    let backend = FakeBackend::open();
    let _lookup = QueuedWordLookup::start(backend.clone(), settings_with_idle(5, 90, 10_000_000, 10));
    let_tasks_run().await;

    tokio::time::advance(Duration::from_secs(11)).await;
    let_tasks_run().await;
    assert_eq!(backend.releases.load(Ordering::SeqCst), 1);

    tokio::time::advance(Duration::from_secs(60)).await;
    let_tasks_run().await;
    assert_eq!(backend.releases.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn a_lookup_restarts_the_idle_timer() {
    let backend = FakeBackend::open();
    let lookup = QueuedWordLookup::start(backend.clone(), settings_with_idle(5, 90, 10_000_000, 10));
    let_tasks_run().await;
    tokio::time::advance(Duration::from_secs(8)).await;
    let_tasks_run().await;

    lookup.lookup("busy").await.unwrap();
    tokio::time::advance(Duration::from_secs(8)).await;
    let_tasks_run().await;
    assert_eq!(backend.releases.load(Ordering::SeqCst), 0);

    tokio::time::advance(Duration::from_secs(3)).await;
    let_tasks_run().await;
    assert_eq!(backend.releases.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn serves_a_lookup_after_the_session_was_released() {
    let backend = FakeBackend::open();
    let lookup = QueuedWordLookup::start(backend.clone(), settings_with_idle(5, 90, 10_000_000, 10));
    let_tasks_run().await;
    tokio::time::advance(Duration::from_secs(11)).await;
    let_tasks_run().await;

    let result = lookup.lookup("later").await.unwrap();

    assert_eq!(result.unwrap().word, "later");
    assert_eq!(backend.releases.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn releases_again_after_a_new_lookup_and_another_idle_period() {
    let backend = FakeBackend::open();
    let lookup = QueuedWordLookup::start(backend.clone(), settings_with_idle(5, 90, 10_000_000, 10));
    let_tasks_run().await;
    tokio::time::advance(Duration::from_secs(11)).await;
    let_tasks_run().await;
    lookup.lookup("again").await.unwrap();

    tokio::time::advance(Duration::from_secs(11)).await;
    let_tasks_run().await;

    assert_eq!(backend.releases.load(Ordering::SeqCst), 2);
}
