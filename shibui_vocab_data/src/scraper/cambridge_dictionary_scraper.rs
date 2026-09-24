use std::path::PathBuf;

use crate::scraper::WordDefinition;
use crate::utils::html_parser::cambridge_parser::CambridgeHtmlParser;
use crate::utils::html_parser::WordNotFoundError;
use crate::utils::http_request::{DefaultHttpRequestMaker, FetchError, HttpRequestMaker};
use crate::utils::html_parser::HtmlParser;

#[derive(Clone)]
pub struct CambridgeDictionaryScraper {
    request_maker: DefaultHttpRequestMaker,
    html_parser: CambridgeHtmlParser,
    storage_dir: PathBuf,
}

impl CambridgeDictionaryScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            request_maker: DefaultHttpRequestMaker::new(),
            html_parser: CambridgeHtmlParser,
            storage_dir: html_storage_dir.into(),
        }
    }

    pub async fn cleanup_stale_flaresolverr_sessions(&self) {
        self.request_maker.cleanup_stale_sessions().await;
    }

    fn make_request_url(&self, word: &str) -> String {
        return format!(
            "{}{}",
            "https://dictionary.cambridge.org/dictionary/english/", word
        );
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

    pub async fn fetch(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
        let cache_path = self.cache_path(word);
        if let Ok(cached_html) = tokio::fs::read_to_string(&cache_path).await {
            return self.parse_result(word, &cached_html);
        }

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

        let result = match self.parse_result(word, &html_content)? {
            Some(result) => result,
            None => return Ok(None),
        };

        if let Some(extracted_html) = &result.extracted_html {
            self.save_extracted_html(&cache_path, extracted_html).await;
        }

        Ok(Some(result))
    }
}
