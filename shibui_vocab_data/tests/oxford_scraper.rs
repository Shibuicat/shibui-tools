use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use shibui_vocab_data::scraper::OxfordScraper;
use shibui_vocab_data::utils::http_request::{FetchError, HttpRequestMaker};

const URL: &str = "https://www.oxfordlearnersdictionaries.com/definition/english/abandon_1";

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

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!("tests/fixtures/oxford_{name}.html")).unwrap()
}

fn scraper(name: &str, responses: &[(&str, Canned)]) -> (OxfordScraper<FakeRequestMaker>, FakeRequestMaker, PathBuf) {
    let maker = FakeRequestMaker::new(responses);
    let dir = std::env::temp_dir().join(format!("shibui_oxford_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    (OxfordScraper::with_request_maker(maker.clone(), &dir), maker, dir)
}

#[tokio::test]
async fn the_first_entry_of_the_word_is_requested() {
    let (scraper, maker, _) = scraper("first", &[(URL, Canned::Html(fixture("abandon")))]);

    let definition = scraper.fetch("Abandon").await.unwrap().unwrap();

    assert_eq!(definition.word, "abandon");
    assert_eq!(maker.requested(), vec![URL.to_owned()]);
}

#[tokio::test]
async fn several_words_are_joined_with_hyphens() {
    let (scraper, maker, _) = scraper("hyphen", &[]);

    let _ = scraper.fetch("ice cream").await;

    assert_eq!(
        maker.requested(),
        vec!["https://www.oxfordlearnersdictionaries.com/definition/english/ice-cream_1".to_owned()]
    );
}

#[tokio::test]
async fn a_404_is_not_found_not_an_error() {
    let (scraper, _, _) = scraper("missing", &[(URL, Canned::NotFound)]);

    assert!(scraper.fetch("abandon").await.unwrap().is_none());
}

#[tokio::test]
async fn the_spelling_suggestion_page_is_not_found() {
    let (scraper, _, _) = scraper("suggest", &[(URL, Canned::Html(fixture("not_found")))]);

    assert!(scraper.fetch("abandon").await.unwrap().is_none());
}

#[tokio::test]
async fn a_changed_layout_is_an_error_so_the_word_is_retried_later() {
    let (scraper, _, _) = scraper("layout", &[(URL, Canned::Html(fixture("layout_changed")))]);

    assert!(scraper.fetch("abandon").await.is_err());
}

#[tokio::test]
async fn a_network_failure_is_an_error() {
    let (scraper, _, _) = scraper("broken", &[(URL, Canned::Broken)]);

    assert!(scraper.fetch("abandon").await.is_err());
}

#[tokio::test]
async fn the_kept_fragment_is_stored_and_used_instead_of_a_second_request() {
    let (scraper, maker, dir) = scraper("cache", &[(URL, Canned::Html(fixture("abandon")))]);

    scraper.fetch("abandon").await.unwrap().unwrap();
    let again = scraper.fetch("abandon").await.unwrap().unwrap();

    assert!(dir.join("abandon.html").exists());
    assert_eq!(maker.requested().len(), 1);
    assert_eq!(again.word, "abandon");
}
