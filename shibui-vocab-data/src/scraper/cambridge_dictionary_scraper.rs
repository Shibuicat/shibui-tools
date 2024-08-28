use super::scraper::{Scraper, WordDefinition};
use crate::utils::{html_parser::HtmlParser, http_request::HttpRequestMaker};
// use serde::Deserialize;

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

    // fn make_suggest_request_url(&self, word: &str) -> String {
    //     format!(
    //         "{}{}",
    //         "https://dictionary.cambridge.org/autocomplete/amp?dataset=english&q=", word
    //     )
    // }

    // async fn fetch_suggestion(&self, word: &str) -> Result<WordSuggestion> {
    //     let url = self.make_suggest_request_url(word);
    //     let result = self.request_maker.get(&url).await?;
    //     let result = serde_json::from_str::<WordSuggestion>(&result)?;
    //     Ok(result)
    // }
}

impl<T: HttpRequestMaker, T2: HtmlParser<Output = WordDefinition>> Scraper
    for CambridgeDictionaryScraper<T, T2>
{
    async fn fetch(&self, word: &str) -> anyhow::Result<Option<WordDefinition>> {
        let fetch_url = self.make_request_url(word);
        dbg!(&fetch_url);
        // let html_content = self.request_maker.get(&fetch_url).await?;

        // dbg!(html_content);
        // let result = self.html_parser.parse(&html_content)?;

        Ok(None)
        // Ok(Some(result))
    }
}

// #[derive(Deserialize)]
// struct WordSuggestion {
//     pub word: String,
//     pub url: String,
//     pub beta: bool,
// }
