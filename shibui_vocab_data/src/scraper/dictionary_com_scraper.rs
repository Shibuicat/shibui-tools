use std::path::PathBuf;

use super::page_scraper::{percent_encode, PageScraper};
use crate::utils::html_parser::dictionary_com_parser::DictionaryComHtmlParser;
use crate::utils::http_request::{HttpRequestMaker, PlainHttpRequestMaker};

const PAGE_URL: &str = "https://www.dictionary.com/browse/";

pub type DictionaryComScraper<R = PlainHttpRequestMaker> = PageScraper<R, DictionaryComHtmlParser>;

impl DictionaryComScraper {
    pub fn new(html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_request_maker(PlainHttpRequestMaker::new(), html_storage_dir)
    }
}

impl<R: HttpRequestMaker> DictionaryComScraper<R> {
    pub fn with_request_maker(request_maker: R, html_storage_dir: impl Into<PathBuf>) -> Self {
        Self::with_parts(request_maker, DictionaryComHtmlParser, page_url, html_storage_dir)
    }
}

fn page_url(word: &str) -> String {
    let slug = word.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    format!("{PAGE_URL}{}", percent_encode(&slug))
}
