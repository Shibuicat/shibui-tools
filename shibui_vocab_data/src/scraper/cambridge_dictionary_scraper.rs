use std::path::PathBuf;

use crate::scraper::WordDefinition;
use crate::utils::html_parser::cambridge_parser::CambridgeHtmlParser;
use crate::utils::html_parser::WordNotFoundError;
use crate::utils::http_request::{DefaultHttpRequestMaker, FetchError, HttpRequestMaker};
use crate::utils::html_parser::HtmlParser;

#[derive(Clone)]
pub struct CambridgeDictionaryScraper<R = DefaultHttpRequestMaker> {
    request_maker: R,
    html_parser: CambridgeHtmlParser,
    storage_dir: PathBuf,
}

impl CambridgeDictionaryScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_request_maker(DefaultHttpRequestMaker::new(), html_storage_dir)
    }

    pub async fn start_flaresolverr_session(&self) {
        self.request_maker.start_session().await;
    }

    pub async fn recycle_flaresolverr_session(&self) {
        self.request_maker.recycle_session().await;
    }
}

impl<R: HttpRequestMaker> CambridgeDictionaryScraper<R> {
    pub fn with_request_maker(request_maker: R, html_storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            request_maker,
            html_parser: CambridgeHtmlParser,
            storage_dir: html_storage_dir.into(),
        }
    }

    fn candidate_urls(&self, word: &str) -> [String; 2] {
        [
            format!("https://dictionary.cambridge.org/search/direct/?datasetsearch=english&q={word}"),
            format!("https://dictionary.cambridge.org/dictionary/english/{word}"),
        ]
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

    async fn fetch_from(&self, word: &str, url: &str) -> anyhow::Result<Option<WordDefinition>> {
        match self.request_maker.get(url).await {
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

    async fn fetch_live(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
        for url in self.candidate_urls(word) {
            if let Some(definition) = self.fetch_from(word, &url).await? {
                return Ok(Some(definition));
            }
        }
        Ok(None)
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
