use anyhow::Result;

pub trait Scraper {
    async fn fetch(&self, word: &str) -> Result<Option<WordDefinition>>;
}

pub struct WordDefinition {
    pub word: String,
    pub uk_pronounce_link: String,
    pub us_pronounce_link: String,
    pub classes: Vec<WordClass>,
}

pub struct WordClass {
    pub class: Class,
    pub definitions: Vec<ClassDefinition>,
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
