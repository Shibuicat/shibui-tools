use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;

use axum::{routing::get, Router};
use shibui_vocab_data::lookup_queue::{LookupBackend, LookupSettings, QueuedWordLookup, WordLookup};
use shibui_vocab_data::scraper::page_scraper::PageScraper;
use shibui_vocab_data::scraper::{CambridgeDictionaryScraper, OxfordScraper, WiktionaryScraper, WordDefinition};
use shibui_vocab_data::utils::html_parser::HtmlParser;
use shibui_vocab_data::utils::http_request::HttpRequestMaker;
mod routes;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let html_storage_dir =
        std::env::var("HTML_STORAGE_DIR").unwrap_or_else(|_| "./html_storage".to_string());
    println!("Storing fetched HTML under {html_storage_dir}");

    let fetcher = Arc::new(Fetcher::new(CambridgeDictionaryScraper::new(html_storage_dir)));
    fetcher.start_session().await;

    let wiktionary_storage_dir = std::env::var("WIKTIONARY_HTML_STORAGE_DIR")
        .unwrap_or_else(|_| "./html_storage_wiktionary".to_string());
    println!("Storing fetched Wiktionary HTML under {wiktionary_storage_dir}");
    let wiktionary = Arc::new(PageFetcher::new(WiktionaryScraper::new(wiktionary_storage_dir)));

    let oxford_storage_dir =
        std::env::var("OXFORD_HTML_STORAGE_DIR").unwrap_or_else(|_| "./html_storage_oxford".to_string());
    println!("Storing fetched Oxford HTML under {oxford_storage_dir}");
    let oxford = Arc::new(PageFetcher::new(OxfordScraper::new(oxford_storage_dir)));

    let settings = LookupSettings::from_env();
    let lookups: HashMap<String, Box<dyn WordLookup>> = HashMap::from([
        (routes::cambridge_dic::DEFAULT_SOURCE.to_owned(), Box::new(QueuedWordLookup::start(fetcher, settings)) as Box<dyn WordLookup>),
        ("wiktionary".to_owned(), Box::new(QueuedWordLookup::start(wiktionary, settings)) as Box<dyn WordLookup>),
        ("oxford".to_owned(), Box::new(QueuedWordLookup::start(oxford, settings)) as Box<dyn WordLookup>),
    ]);
    let shared_state = Arc::new(AppState { lookups });

    let app = Router::new().route(
        "/query",
        get({
            let shared_state = Arc::clone(&shared_state);
            move |query| routes::cambridge_dic::get_word(query, shared_state)
        }),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Listening on 3001");
    axum::serve(listener, app).await.unwrap();
}

pub struct AppState {
    lookups: HashMap<String, Box<dyn WordLookup>>,
}

pub struct PageFetcher<R, P> {
    scraper: PageScraper<R, P>,
}

impl<R, P> PageFetcher<R, P> {
    pub fn new(scraper: PageScraper<R, P>) -> Self {
        Self { scraper }
    }
}

impl<R, P> LookupBackend for PageFetcher<R, P>
where
    R: HttpRequestMaker + Send + Sync + 'static,
    P: HtmlParser<Output = WordDefinition> + Send + Sync + 'static,
{
    fn fetch(&self, word: &str) -> impl Future<Output = anyhow::Result<Option<WordDefinition>>> + Send {
        self.scraper.fetch(word)
    }

    fn rotate_session(&self) -> impl Future<Output = ()> + Send {
        async {}
    }

    fn release_session(&self) -> impl Future<Output = ()> + Send {
        async {}
    }
}

pub struct Fetcher {
    scraper: CambridgeDictionaryScraper,
}

impl Fetcher {
    pub fn new(scraper: CambridgeDictionaryScraper) -> Self {
        Self { scraper }
    }

    pub async fn start_session(&self) {
        self.scraper.start_flaresolverr_session().await;
    }
}

impl LookupBackend for Fetcher {
    fn fetch(&self, word: &str) -> impl Future<Output = anyhow::Result<Option<WordDefinition>>> + Send {
        self.scraper.fetch(word)
    }

    fn rotate_session(&self) -> impl Future<Output = ()> + Send {
        self.scraper.recycle_flaresolverr_session()
    }

    fn release_session(&self) -> impl Future<Output = ()> + Send {
        self.scraper.release_flaresolverr_session()
    }
}
