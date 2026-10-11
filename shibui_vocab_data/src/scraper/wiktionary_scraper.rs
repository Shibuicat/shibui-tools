use std::path::PathBuf;

use crate::scraper::WordDefinition;
use crate::utils::html_parser::wiktionary_parser::WiktionaryHtmlParser;
use crate::utils::html_parser::{HtmlParser, WordNotFoundError};
use crate::utils::http_request::{FetchError, HttpRequestMaker, PlainHttpRequestMaker};

const PAGE_URL: &str = "https://en.wiktionary.org/api/rest_v1/page/html/";

#[derive(Clone)]
pub struct WiktionaryScraper<R = PlainHttpRequestMaker> {
    request_maker: R,
    html_parser: WiktionaryHtmlParser,
    storage_dir: PathBuf,
}

impl WiktionaryScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_request_maker(PlainHttpRequestMaker::new(), html_storage_dir)
    }
}

impl<R: HttpRequestMaker> WiktionaryScraper<R> {
    pub fn with_request_maker(request_maker: R, html_storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            request_maker,
            html_parser: WiktionaryHtmlParser,
            storage_dir: html_storage_dir.into(),
        }
    }

    fn page_url(word: &str) -> String {
        format!("{PAGE_URL}{}", percent_encode(word))
    }

    fn cache_path(&self, word: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.html", word.to_lowercase()))
    }

    fn parse_result(&self, word: &str, html: &str) -> anyhow::Result<Option<WordDefinition>> {
        match self.html_parser.parse(html) {
            Ok(result) => Ok(Some(result)),
            Err(err) if err.downcast_ref::<WordNotFoundError>().is_some() => {
                eprintln!("{word} doesn't exist: {err}");
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }

    async fn save_extracted_html(&self, path: &std::path::Path, html: &str) {
        if let Some(parent) = path.parent() {
            if let Err(err) = tokio::fs::create_dir_all(parent).await {
                eprintln!("Failed to create HTML storage dir {parent:?}: {err}");
                return;
            }
        }
        if let Err(err) = tokio::fs::write(path, html).await {
            eprintln!("Failed to save extracted HTML to {path:?}: {err}");
        }
    }

    async fn fetch_live(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
        match self.request_maker.get(&Self::page_url(word)).await {
            Ok(html) => self.parse_result(word, &html),
            Err(FetchError::NotFound(msg)) => {
                eprintln!("{word} doesn't exist: {msg}");
                Ok(None)
            }
            Err(FetchError::Other(err)) => {
                eprintln!("Failed to fetch {word}: {err}");
                Err(err)
            }
        }
    }

    pub async fn fetch(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
        let cache_path = self.cache_path(word);
        if let Ok(cached_html) = tokio::fs::read_to_string(&cache_path).await {
            return self.parse_result(word, &cached_html);
        }

        let result = match self.fetch_live(word).await? {
            Some(result) => result,
            None => return Ok(None),
        };
        if let Some(extracted_html) = &result.extracted_html {
            self.save_extracted_html(&cache_path, extracted_html).await;
        }
        Ok(Some(result))
    }
}

fn percent_encode(word: &str) -> String {
    word.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
