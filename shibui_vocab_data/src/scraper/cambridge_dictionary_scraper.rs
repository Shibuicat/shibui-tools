use std::path::PathBuf;

use crate::scraper::WordDefinition;
use crate::utils::html_parser::cambridge_parser::CambridgeHtmlParser;
use crate::utils::http_request::{DefaultHttpRequestMaker, FetchError, FileBackedHttpRequestMaker};
use crate::utils::{html_parser::HtmlParser, http_request::HttpRequestMaker};

#[derive(Clone)]
pub struct CambridgeDictionaryScraper {
    request_maker: FileBackedHttpRequestMaker<DefaultHttpRequestMaker>,
    html_parser: CambridgeHtmlParser,
}

impl CambridgeDictionaryScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            request_maker: FileBackedHttpRequestMaker::new(DefaultHttpRequestMaker::new(), html_storage_dir),
            html_parser: CambridgeHtmlParser,
        }
    }

    fn make_request_url(&self, word: &str) -> String {
        return format!(
            "{}{}",
            "https://dictionary.cambridge.org/dictionary/english/", word
        );
    }

    pub async fn fetch(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
        let fetch_url = self.make_request_url(word);
        let html_content = match self.request_maker.get(&fetch_url).await {
            Ok(html) => html,
            Err(FetchError::NotFound(msg)) => {
                eprintln!("{word} doesn't exist: {msg}");
                return Ok(None);
            }
            Err(FetchError::Other(err)) => {
                eprintln!("Failed to fetch {word}: {err}");
                return Err(err);
            }
        };
        let result = self.html_parser.parse(&html_content)?;

        Ok(Some(result))
    }
}
