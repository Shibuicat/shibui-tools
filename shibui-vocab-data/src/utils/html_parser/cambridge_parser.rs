use anyhow::{anyhow, Result};
use scraper::{ElementRef, Html, Selector};

use crate::{
    scraper::scraper::{WordClass, WordDefinition, WordPronounce},
    utils::html_parser::{cambridge_elements::WordPage, cambridge_parser},
};

use super::HtmlParser;

pub struct CambridgeHtmlParser {}

impl CambridgeHtmlParser {
    pub fn new() -> Self {
        Self {}
    }

    fn get_word_classes_selector() -> anyhow::Result<Selector> {
        let selector_parse_result =
            Selector::parse(".pr.entry-body__el").map_err(|err| err.to_string());
        match selector_parse_result {
            Ok(selector) => Ok(selector),
            Err(err) => Err(anyhow!(err)),
        }
    }
}

impl HtmlParser for CambridgeHtmlParser {
    type Output = WordDefinition;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        //word class class="pr entry-body__el"
        // let uk_pronounce_selector = Selector::parse(".uk.dpron-i");
        // let us_pronounce_selector = Selector::parse(".us.dpron-i");
        let html_doc = Html::parse_document(html_content);
        let word_page = WordPage::new(&html_doc);
        word_page.
    }
}
