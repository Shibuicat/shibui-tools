pub mod scraper;
use scraper::scraper::Scraper;
mod utils;

use anyhow::Result;
use scraper::cambridge_dictionary_scraper::CambridgeDictionaryScraper;
use utils::{
    html_parser::cambridge_parser::CambridgeHtmlParser, http_request::DefaultHttpRequestMaker,
};

#[tokio::main]
async fn main() -> Result<()> {
    let request_maker = DefaultHttpRequestMaker::new();
    let html_parser = CambridgeHtmlParser::new();
    let fetcher = CambridgeDictionaryScraper::new(request_maker, html_parser);
    let word_definition = fetcher.fetch("hello").await?;
    dbg!(word_definition);
    Ok(())
}
