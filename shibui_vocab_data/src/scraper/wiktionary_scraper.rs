use std::path::PathBuf;

use super::page_scraper::{percent_encode, PageScraper};
use crate::utils::html_parser::wiktionary_parser::WiktionaryHtmlParser;
use crate::utils::http_request::{HttpRequestMaker, PlainHttpRequestMaker};

const PAGE_URL: &str = "https://en.wiktionary.org/api/rest_v1/page/html/";

pub type WiktionaryScraper<R = PlainHttpRequestMaker> = PageScraper<R, WiktionaryHtmlParser>;

impl WiktionaryScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_request_maker(PlainHttpRequestMaker::new(), html_storage_dir)
    }
}

impl<R: HttpRequestMaker> WiktionaryScraper<R> {
    pub fn with_request_maker(request_maker: R, html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_parts(request_maker, WiktionaryHtmlParser, page_url, html_storage_dir)
    }
}

fn page_url(word: &str) -> String {
    format!("{PAGE_URL}{}", percent_encode(word))
}
