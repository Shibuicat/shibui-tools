pub mod scraper;
mod utils;
use anyhow::Result;
use scraper::cambridge_dictionary_scraper::CambridgeDictionaryScraper;

#[tokio::main]
async fn main() -> Result<()> {
    let fetcher = CambridgeDictionaryScraper::new();
    todo!()
}
