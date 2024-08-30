use anyhow::Result;

pub trait Scraper {
    async fn fetch(&self, word: &str) -> Result<Option<WordDefinition>>;
}

#[derive(Debug)]
pub struct WordDefinition {
    pub word: String,
    pub classes: Vec<WordClass>,
}

#[derive(Debug)]
pub struct WordPronounce {
    pub region: Region,
    pub ipa: String,
    pub link: String,
}

#[derive(Debug)]
pub enum Region {
    US,
    UK,
}

#[derive(Debug)]
pub struct WordClass {
    pub class: Class,
    pub definitions: Vec<ClassDefinition>,
    pub pronounces: Vec<WordPronounce>,
}

#[derive(Debug)]
pub struct ClassDefinition {
    pub short_meaning: Option<String>,
    pub explaination: String,
    pub example: Option<Vec<WordUsageExample>>,
}

#[derive(Debug)]
pub enum Class {
    Noun,
    Verb,
    Adverb,
    Adjective,
    Pronounce,
}

#[derive(Debug)]
pub struct WordUsageExample {
    pub example_sentence: String,
    pub meaning: String,
}
