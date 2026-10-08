use std::future::Future;
use std::sync::Arc;

use axum::{routing::get, Router};
use shibui_vocab_data::lookup_queue::{LookupBackend, LookupSettings, QueuedWordLookup, WordLookup};
use shibui_vocab_data::scraper::{CambridgeDictionaryScraper, WordDefinition};
mod routes;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let html_storage_dir =
        std::env::var("HTML_STORAGE_DIR").unwrap_or_else(|_| "./html_storage".to_string());
    println!("Storing fetched HTML under {html_storage_dir}");

    let fetcher = Arc::new(Fetcher::new(CambridgeDictionaryScraper::new(html_storage_dir)));
    fetcher.start_session().await;
    let lookup = QueuedWordLookup::start(fetcher, LookupSettings::from_env());
    let shared_state = Arc::new(AppState {
        lookup: Box::new(lookup),
    });

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
    lookup: Box<dyn WordLookup>,
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
