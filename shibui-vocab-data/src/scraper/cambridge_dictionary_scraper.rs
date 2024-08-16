use super::scraper::{Scraper, WordDefinition};
use anyhow::Result;

pub struct CambridgeDictionaryScraper {
    request_maker: HttpRequestMaker,
}

impl CambridgeDictionaryScraper {
    pub fn new() -> Self {}
}

impl Scraper for CambridgeDictionaryScraper {
    async fn fetch(word: String) -> Result<Option<WordDefinition>> {
        todo!()
    }
}
