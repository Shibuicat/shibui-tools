use std::path::PathBuf;

use super::page_scraper::{percent_encode, PageScraper};
use crate::utils::html_parser::collins_parser::CollinsHtmlParser;
use crate::utils::http_request::{ChallengeHttpRequestMaker, HttpRequestMaker};

pub const WARM_UP_URL: &str = "https://www.collinsdictionary.com/";
const PAGE_URL: &str = "https://www.collinsdictionary.com/dictionary/english/";

pub type CollinsScraper<R = ChallengeHttpRequestMaker> = PageScraper<R, CollinsHtmlParser>;

impl CollinsScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_request_maker(ChallengeHttpRequestMaker::new(WARM_UP_URL), html_storage_dir)
    }
}

impl<R: HttpRequestMaker> CollinsScraper<R> {
    pub fn with_request_maker(request_maker: R, html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_parts(request_maker, CollinsHtmlParser, page_url, html_storage_dir)
    }
}

fn page_url(word: &str) -> String {
    let slug = word.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    format!("{PAGE_URL}{}", percent_encode(&slug))
}
