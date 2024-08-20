use crate::utils::{
    html_parser::{self, HtmlParser},
    http_request::HttpRequestMaker,
};

use super::scraper::{Scraper, WordDefinition};
use anyhow::Result;

pub struct CambridgeDictionaryScraper<T: HttpRequestMaker, T2: HtmlParser> {
    request_maker: T,
    html_parser: T2,
}

impl<T: HttpRequestMaker, T2: HtmlParser> CambridgeDictionaryScraper<T, T2> {
    pub fn new(request_maker: T, html_parser: T2) -> Self {
        Self {
            request_maker,
            html_parser,
        }
    }
}

impl<T: HttpRequestMaker, T2: HtmlParser> Scraper for CambridgeDictionaryScraper<T, T2> {
    async fn fetch(&self, word: String) -> anyhow::Result<Option<WordDefinition>> {
        let html_content = self
            .request_maker
            .get("https://dictionary.cambridge.org/")
            .await?;

        let result = self.html_parser.parse::<WordDefinition>(&html_content)?;

        Ok(Some(result))
    }
}
