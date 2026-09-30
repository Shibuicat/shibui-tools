use anyhow::bail;
use scraper::Html;

use crate::{
    scraper::WordDefinition,
    utils::html_parser::{cambridge_elements::WordPage, WordNotFoundError},
};

use super::HtmlParser;

#[derive(Clone)]
pub struct CambridgeHtmlParser;

impl HtmlParser for CambridgeHtmlParser {
    type Output = WordDefinition;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        let html_doc = Html::parse_document(html_content);
        let word_page = WordPage::new(&html_doc);
        let word = word_page?.get_word_definition()?;
        if !word.has_meanings() {
            bail!(WordNotFoundError);
        }
        Ok(word)
    }
}
