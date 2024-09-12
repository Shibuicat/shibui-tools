pub mod scraper;
mod utils;

use anyhow::Result;
use scraper::CambridgeDictionaryScraper;
use scraper::Scraper;
use utils::{
    html_parser::cambridge_parser::CambridgeHtmlParser, http_request::DefaultHttpRequestMaker,
};

#[tokio::main]
async fn main() -> Result<()> {
    let request_maker = DefaultHttpRequestMaker::new();
    let fetcher = CambridgeDictionaryScraper::new(request_maker, CambridgeHtmlParser);
    let word_definition = fetcher.fetch("hello").await?;
    dbg!(word_definition);
    Ok(())
}
