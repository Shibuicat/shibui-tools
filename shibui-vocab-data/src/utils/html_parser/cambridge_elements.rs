use scraper::{ElementRef, Html, Selector};
use anyhow::{anyhow, bail};

use crate::scraper::scraper::WordDefinition;

pub struct WordPage<'a> {
    pub content: &'a Html,
}

impl<'a> WordPage<'a> {
    //There is a possibility that the word does not exist.
    pub fn new(html: &Html) -> anyhow::Result<Self> {}

    fn word_class_sections(&self) -> Vec<WordClassSection<'a>> {
        let selector = Selector::parse(".pr.entry-body__el").unwrap();
        self.content
            .select(&selector)
            .map(|ele| WordClassSection::new(ele, &self))
            .collect()
    }

    pub fn get_word_definition(&self) -> anyhow::Result<WordDefinition> {
        let word_classes = self.word_class_sections();
        if word_classes.len() == 0{
            bail!("Word not found");
        }
        
        let word = word_classes.first().get_current_word();
        let word_definition = WordDefinition {
            word: word,
            classes: word_classes.map(|class|{
               class. 
            })
        };

        Ok(word_definition)
    }
}

pub struct WordClassSection<'a> {
    //the word class ele
    pub inner_html_ele: ElementRef<'a>,
    word_page: &'a WordPage<'a>,
}

impl<'a> WordClassSection<'a> {
    pub fn new(inner_html_ele: ElementRef, word_page: &WordPage) -> Self {
        Self {
            word_page,
            inner_html_ele,
        }
    }

    pub fn get_current_word() -> String {
        "".to_owned()
    }
}

pub struct WordClassHeaderSection<'a> {
    pub inner_html_ele: &'a ElementRef<'a>,
    word_class: &'a WordClassSection<'a>,
}

impl<'a> WordClassHeaderSection<'a> {
    pub fn new(inner_html_ele: &ElementRef, word_class: &WordClassSection) -> Self {
        Self {
            inner_html_ele,
            word_class,
        }
    }
}
