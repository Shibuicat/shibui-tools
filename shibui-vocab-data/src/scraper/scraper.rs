use std::future::Future;

use anyhow::Result;

pub trait Scraper {
    fn fetch(&self, word: &str) -> impl Future<Output = Result<Option<WordDefinition>>>;
}

pub struct WordDefinition {
    word: String,
    uk_pronounce_link: String,
    us_pronounce_link: String,
    type_definition: Vec<WordTypeDefinition>,
}

pub struct WordTypeDefinition {
    pub word_type: WordType,
    pub example: Option<Vec<WordUsageExample>>,
}

pub enum WordType {
    Noun,
    Verb,
    Adverb,
    Adjective,
    Pronounce,
}

pub struct WordUsageExample {
    pub example_sentence: String,
    pub meaning: String,
}
