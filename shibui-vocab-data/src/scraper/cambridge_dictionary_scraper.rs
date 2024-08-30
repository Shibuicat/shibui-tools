use super::scraper::{Scraper, WordDefinition};
use crate::utils::{html_parser::HtmlParser, http_request::HttpRequestMaker};

pub struct CambridgeDictionaryScraper<T: HttpRequestMaker, T2: HtmlParser<Output = WordDefinition>>
{
    request_maker: T,
    html_parser: T2,
}

impl<T: HttpRequestMaker, T2: HtmlParser<Output = WordDefinition>>
    CambridgeDictionaryScraper<T, T2>
{
    pub fn new(request_maker: T, html_parser: T2) -> Self {
        Self {
            request_maker,
            html_parser,
        }
    }

    fn make_request_url(&self, word: &str) -> String {
        return format!(
            "{}{}",
            "https://dictionary.cambridge.org/dictionary/english/", word
        );
    }
}

impl<T: HttpRequestMaker, T2: HtmlParser<Output = WordDefinition>> Scraper
    for CambridgeDictionaryScraper<T, T2>
{
    async fn fetch(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
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
