use anyhow::Result;
use std::collections::HashMap;

pub trait Scraper {
    async fn fetch(&self, word: String) -> Result<Option<WordDefinition>>;
}

pub struct WordDefinition {
    word: String,
    word_type: Vec<WordType>,
}

pub struct WordType {
    pub word_type: WordTypeType,
    pub example: Option<HashMap<WordTypeType, Option<Vec<WordUsageExample>>>>,
}

pub enum WordTypeType {
    Noun,
    Verb,
    Adverb,
    Adjective,
    Pronounce,
}

pub struct WordUsageExample {
    pub sentence: String,
    pub meaning: String,
}
