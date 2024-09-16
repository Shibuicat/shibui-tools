use crate::scraper::WordDefinition;
use crate::utils::html_parser::cambridge_parser::CambridgeHtmlParser;
use crate::utils::http_request::DefaultHttpRequestMaker;
use crate::utils::{html_parser::HtmlParser, http_request::HttpRequestMaker};

#[derive(Clone)]
pub struct CambridgeDictionaryScraper {
    request_maker: DefaultHttpRequestMaker,
    html_parser: CambridgeHtmlParser,
}

impl CambridgeDictionaryScraper {
    pub fn new() -> Self {
        Self {
            request_maker: DefaultHttpRequestMaker::new(),
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
        let fetch_result = self.request_maker.get(&fetch_url).await;
        if fetch_result.is_err() {
            println!("Word {word} doesn't exist");
        }
        let html_content = fetch_result?;
        let result = self.html_parser.parse(&html_content)?;

        Ok(Some(result))
    }
}
