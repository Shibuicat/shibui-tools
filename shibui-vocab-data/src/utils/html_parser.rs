use anyhow::{anyhow, Result};
use scraper::{element_ref, selectable::Selectable, ElementRef, Html, Selector};

use crate::scraper::scraper::{WordClass, WordDefinition};

pub trait HtmlParser {
    type Output;
    fn parse(&self, html_content: &str, word: &str) -> anyhow::Result<Self::Output>;
}

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

    fn get_word_classes(&self, word_class_element: ElementRef) -> Result<WordClass> {
        todo!()
    }
}

impl HtmlParser for CambridgeHtmlParser {
    type Output = WordDefinition;
    fn parse(&self, html_content: &str, word: &str) -> anyhow::Result<Self::Output> {
        // class="pr entry-body__el"
        let html_doc = Html::parse_document(html_content);
        let selector = Self::get_word_classes_selector()?;
        let mut word_classes = vec![]

        for ele in html_doc.select(&selector) {
            match self.get_word_classes(ele){
                Ok(word_class) => word_classes.push(word_class),
                Err(err) => return Err(err)
            }
        }

        let pronounce_link_selector = Selector::parse("");
        
        let word_definition = WordDefinition{
            word: word.to_owned(),
            classes: word_classes,
            uk_pronounce_link: "".to_owned(),
            us_pronounce_link: "".to_owned(), 
        };
        Ok(word_definition)
    }
}
