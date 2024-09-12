use axum::extract::Query;
use shibui_vocab_data::scraper::CambridgeDictionaryScraper;

pub struct Fetcher {
    scraper: CambridgeDictionaryScraper,
}

impl Fetcher {
    pub fn new(scraper: CambridgeDictionaryScraper) -> Self {
        Self { scraper }
    }
    pub async fn get_word(query: Query<String>) -> String {}
}
