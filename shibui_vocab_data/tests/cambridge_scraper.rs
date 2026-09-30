use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use shibui_vocab_data::scraper::CambridgeDictionaryScraper;
use shibui_vocab_data::utils::http_request::{FetchError, HttpRequestMaker};

const SEARCH_URL: &str = "https://dictionary.cambridge.org/search/direct/?datasetsearch=english&q=abased";
const DIRECT_URL: &str = "https://dictionary.cambridge.org/dictionary/english/abased";

#[derive(Clone)]
enum Canned {
    Html(String),
    NotFound,
    Broken,
}

#[derive(Clone)]
struct FakeRequestMaker {
    responses: HashMap<String, Canned>,
    requested: Arc<Mutex<Vec<String>>>,
}

impl FakeRequestMaker {
    fn new(responses: &[(&str, Canned)]) -> Self {
        Self {
            responses: responses
                .iter()
                .map(|(url, canned)| (url.to_string(), canned.clone()))
                .collect(),
            requested: Arc::new(Mutex::new(vec![])),
        }
    }

    fn requested(&self) -> Vec<String> {
        self.requested.lock().unwrap().clone()
    }
}

impl HttpRequestMaker for FakeRequestMaker {
    async fn get(&self, url: &str) -> Result<String, FetchError> {
        self.requested.lock().unwrap().push(url.to_string());
        match self.responses.get(url) {
            Some(Canned::Html(html)) => Ok(html.clone()),
            Some(Canned::Broken) => Err(FetchError::Other(anyhow::anyhow!("connection reset"))),
            _ => Err(FetchError::NotFound(format!("{url} not found"))),
        }
    }
}

fn entry_html() -> String {
    std::fs::read_to_string("tests/fixtures/accede.html").unwrap()
}

fn empty_page() -> String {
    "<html><body>No entry</body></html>".to_string()
}

fn storage_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shibui_scraper_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn scraper(name: &str, responses: &[(&str, Canned)]) -> (CambridgeDictionaryScraper<FakeRequestMaker>, FakeRequestMaker, PathBuf) {
    let maker = FakeRequestMaker::new(responses);
    let dir = storage_dir(name);
    (CambridgeDictionaryScraper::with_request_maker(maker.clone(), &dir), maker, dir)
}

#[tokio::test]
async fn search_result_is_used_and_direct_url_is_not_requested() {
    let (scraper, maker, _) = scraper("search_first", &[(SEARCH_URL, Canned::Html(entry_html()))]);

    let definition = scraper.fetch("abased").await.unwrap();

    assert!(definition.is_some());
    assert_eq!(maker.requested(), vec![SEARCH_URL]);
}

#[tokio::test]
async fn direct_url_is_the_fallback_when_search_finds_nothing() {
    let (scraper, maker, _) = scraper(
        "fallback",
        &[(SEARCH_URL, Canned::NotFound), (DIRECT_URL, Canned::Html(entry_html()))],
    );

    let definition = scraper.fetch("abased").await.unwrap();

    assert!(definition.is_some());
    assert_eq!(maker.requested(), vec![SEARCH_URL, DIRECT_URL]);
}

#[tokio::test]
async fn direct_url_is_the_fallback_when_the_search_page_has_no_entry() {
    let (scraper, maker, _) = scraper(
        "fallback_empty_page",
        &[(SEARCH_URL, Canned::Html(empty_page())), (DIRECT_URL, Canned::Html(entry_html()))],
    );

    assert!(scraper.fetch("abased").await.unwrap().is_some());
    assert_eq!(maker.requested(), vec![SEARCH_URL, DIRECT_URL]);
}

#[tokio::test]
async fn word_is_not_found_only_when_both_urls_find_nothing() {
    let (scraper, maker, _) = scraper("both_missing", &[]);

    assert!(scraper.fetch("abased").await.unwrap().is_none());
    assert_eq!(maker.requested(), vec![SEARCH_URL, DIRECT_URL]);
}

#[tokio::test]
async fn fetch_failure_is_an_error_not_a_not_found() {
    let (scraper, _, _) = scraper("broken", &[(SEARCH_URL, Canned::Broken)]);

    assert!(scraper.fetch("abased").await.is_err());
}

#[tokio::test]
async fn found_word_is_cached_and_served_without_another_request() {
    let (scraper, maker, dir) = scraper("cache", &[(SEARCH_URL, Canned::Html(entry_html()))]);

    scraper.fetch("abased").await.unwrap();
    let second = scraper.fetch("abased").await.unwrap();

    assert!(second.is_some());
    assert_eq!(maker.requested().len(), 1);
    assert!(std::fs::read_to_string(dir.join("abased.html")).unwrap().starts_with("<div"));
}

#[tokio::test]
async fn not_found_word_is_not_cached() {
    let (scraper, _, dir) = scraper("not_cached", &[]);

    scraper.fetch("abased").await.unwrap();

    assert!(!dir.join("abased.html").exists());
}
