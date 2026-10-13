use std::path::PathBuf;

use super::page_scraper::{percent_encode, PageScraper};
use crate::utils::html_parser::oxford_parser::OxfordHtmlParser;
use crate::utils::http_request::{HttpRequestMaker, PlainHttpRequestMaker};

const PAGE_URL: &str = "https://www.oxfordlearnersdictionaries.com/definition/english/";
const FIRST_ENTRY_SUFFIX: &str = "_1";

pub type OxfordScraper<R = PlainHttpRequestMaker> = PageScraper<R, OxfordHtmlParser>;

impl OxfordScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_request_maker(PlainHttpRequestMaker::new(), html_storage_dir)
    }
}

impl<R: HttpRequestMaker> OxfordScraper<R> {
    pub fn with_request_maker(request_maker: R, html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_parts(request_maker, OxfordHtmlParser, page_url, html_storage_dir)
    }
}

fn page_url(word: &str) -> String {
    let slug = word.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    format!("{PAGE_URL}{}{FIRST_ENTRY_SUFFIX}", percent_encode(&slug))
}
