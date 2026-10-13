use std::path::{Path, PathBuf};

use crate::scraper::WordDefinition;
use crate::utils::html_parser::{HtmlParser, WordNotFoundError};
use crate::utils::http_request::{FetchError, HttpRequestMaker};

#[derive(Clone)]
pub struct PageScraper<R, P> {
    request_maker: R,
    html_parser: P,
    storage_dir: PathBuf,
    page_url: fn(&str) -> String,
}

impl<R: HttpRequestMaker, P: HtmlParser<Output = WordDefinition>> PageScraper<R, P> {
    pub fn with_parts(
        request_maker: R,
        html_parser: P,
        page_url: fn(&str) -> String,
        html_storage_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            request_maker,
            html_parser,
            storage_dir: html_storage_dir.into(),
            page_url,
        }
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

    async fn save_extracted_html(&self, path: &Path, html: &str) {
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
        match self.request_maker.get(&(self.page_url)(word)).await {
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

pub fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (byte as char).to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
