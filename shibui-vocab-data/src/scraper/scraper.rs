use anyhow::Result;

pub trait Scraper {
    async fn fetch(&self, word: &str) -> Result<Option<WordDefinition>>;
}

pub struct WordDefinition {
    pub word: String,
    pub classes: Vec<WordClass>,
}
pub struct WordPronounce {
    pub region: Region,
    pub ipa: String,
    pub link: String,
}

pub enum Region {
    US,
    UK,
}

pub struct WordClass {
    pub class: Class,
    pub definitions: Vec<ClassDefinition>,
    pub pronounces: Vec<WordPronounce>,
}

pub struct ClassDefinition {
    pub short_meaning: Option<String>,
    pub explaination: String,
    pub example: Option<Vec<WordUsageExample>>,
}

pub enum Class {
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
