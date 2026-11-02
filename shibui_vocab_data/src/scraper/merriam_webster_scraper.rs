use std::path::PathBuf;

use super::page_scraper::{percent_encode, PageScraper};
use crate::utils::html_parser::merriam_webster_parser::MerriamWebsterHtmlParser;
use crate::utils::http_request::{ChallengeHttpRequestMaker, HttpRequestMaker};

pub const WARM_UP_URL: &str = "https://www.merriam-webster.com/";
const PAGE_URL: &str = "https://www.merriam-webster.com/dictionary/";

pub type MerriamWebsterScraper<R = ChallengeHttpRequestMaker> = PageScraper<R, MerriamWebsterHtmlParser>;

impl MerriamWebsterScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_request_maker(ChallengeHttpRequestMaker::new(WARM_UP_URL), html_storage_dir)
    }
}

impl<R: HttpRequestMaker> MerriamWebsterScraper<R> {
    pub fn with_request_maker(request_maker: R, html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_parts(request_maker, MerriamWebsterHtmlParser, page_url, html_storage_dir)
    }
}

fn page_url(word: &str) -> String {
    format!("{PAGE_URL}{}", percent_encode(word.trim().to_lowercase().as_str()))
}
