use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use shibui_vocab_data::scraper::WiktionaryScraper;
use shibui_vocab_data::utils::http_request::{FetchError, HttpRequestMaker};

const URL: &str = "https://en.wiktionary.org/api/rest_v1/page/html/beautiful";

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
            responses: responses.iter().map(|(url, canned)| (url.to_string(), canned.clone())).collect(),
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
    std::fs::read_to_string("tests/fixtures/wiktionary_beautiful.html").unwrap()
}

fn storage_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shibui_wiktionary_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn scraper(name: &str, responses: &[(&str, Canned)]) -> (WiktionaryScraper<FakeRequestMaker>, FakeRequestMaker, PathBuf) {
    let maker = FakeRequestMaker::new(responses);
    let dir = storage_dir(name);
    (WiktionaryScraper::with_request_maker(maker.clone(), &dir), maker, dir)
}

#[tokio::test]
async fn a_word_is_fetched_from_the_rest_page_and_parsed() {
    let (scraper, maker, _) = scraper("fetch", &[(URL, Canned::Html(entry_html()))]);

    let definition = scraper.fetch("beautiful").await.unwrap().unwrap();

    assert_eq!(definition.word, "beautiful");
    assert_eq!(maker.requested(), vec![URL.to_owned()]);
}

#[tokio::test]
async fn the_word_is_percent_encoded_in_the_address() {
    let (scraper, maker, _) = scraper("encode", &[]);

    let _ = scraper.fetch("ice cream").await;

    assert_eq!(
        maker.requested(),
        vec!["https://en.wiktionary.org/api/rest_v1/page/html/ice%20cream".to_owned()]
    );
}

#[tokio::test]
async fn a_missing_page_is_not_found_not_an_error() {
    let (scraper, _, _) = scraper("missing", &[(URL, Canned::NotFound)]);

    assert!(scraper.fetch("beautiful").await.unwrap().is_none());
}

#[tokio::test]
async fn a_page_without_an_english_entry_is_not_found() {
    let page = "<section><h2 id=\"French\">French</h2></section>".to_owned();
    let (scraper, _, _) = scraper("french", &[(URL, Canned::Html(page))]);

    assert!(scraper.fetch("beautiful").await.unwrap().is_none());
}

#[tokio::test]
async fn a_network_failure_is_an_error_so_the_word_is_retried_later() {
    let (scraper, _, _) = scraper("broken", &[(URL, Canned::Broken)]);

    assert!(scraper.fetch("beautiful").await.is_err());
}

#[tokio::test]
async fn the_kept_fragment_is_stored_and_used_instead_of_a_second_request() {
    let (scraper, maker, dir) = scraper("cache", &[(URL, Canned::Html(entry_html()))]);

    scraper.fetch("beautiful").await.unwrap().unwrap();
    let again = scraper.fetch("beautiful").await.unwrap().unwrap();

    assert!(dir.join("beautiful.html").exists());
    assert_eq!(maker.requested().len(), 1);
    assert_eq!(again.word, "beautiful");
}
