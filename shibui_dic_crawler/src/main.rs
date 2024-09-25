use std::sync::Arc;

use axum::{routing::get, Router};
use shibui_vocab_data::scraper::{CambridgeDictionaryScraper, WordDefinition};
mod routes;

#[tokio::main]
async fn main() {
    let cambridge_fetcher = Fetcher::new(CambridgeDictionaryScraper::new());
    let shared_state = Arc::new(AppState {
        fetcher: cambridge_fetcher,
    });

    let app = Router::new().route(
        "/query",
        get({
            let shared_state = Arc::clone(&shared_state);
            move |query| routes::cambridge_dic::get_word(query, shared_state)
        }),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
struct AppState {
    fetcher: Fetcher,
}

#[derive(Clone)]
pub struct Fetcher {
    scraper: CambridgeDictionaryScraper,
}

impl Fetcher {
    pub fn new(scraper: CambridgeDictionaryScraper) -> Self {
        Self { scraper }
    }

    pub async fn fetch<T: AsRef<str>>(&self, word: T) -> anyhow::Result<Option<WordDefinition>> {
        self.scraper.fetch(word.as_ref()).await
    }
}
