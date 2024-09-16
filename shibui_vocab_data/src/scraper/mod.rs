pub mod cambridge_dictionary_scraper;
pub use cambridge_dictionary_scraper::CambridgeDictionaryScraper;
use serde::Serialize;
pub mod tratu_coviet;

use std::future::Future;

use anyhow::Result;

pub trait Scraper {
    fn fetch(&self, word: &str) -> impl Future<Output = Result<Option<WordDefinition>>>;
}

#[derive(Debug, Serialize)]
pub struct WordDefinition {
    pub word: String,
    pub classes: Vec<WordClass>,
}

#[derive(Debug, Serialize)]
pub struct WordPronounce {
    pub region: Region,
    pub ipa: String,
    pub link: String,
}

#[derive(Debug, Serialize)]
pub enum Region {
    US,
    UK,
}

#[derive(Debug, Serialize)]
pub struct WordClass {
    pub class_name: Class,
    pub definitions: Vec<ClassDefinition>,
    pub pronounces: Vec<WordPronounce>,
}

#[derive(Debug, Serialize)]
pub struct ClassDefinition {
    pub contexts: Vec<WordContext>,
}

#[derive(Debug, Serialize)]
pub struct WordContext {
    pub description: Option<String>,
    pub meanings: Vec<WordExplanation>,
}

#[derive(Debug, Serialize)]
pub struct WordExplanation {
    pub explanation: String,
    pub examples: Vec<WordUsageExample>,
}

#[derive(Debug, Serialize)]
pub enum Class {
    Noun,
    Verb,
    Adverb,
    Adjective,
    Pronounce,
    Determiner,
    #[serde(untagged)]
    Undefined(String),
}

impl From<&str> for Class {
    fn from(value: &str) -> Self {
        match value {
            "verb" => Self::Verb,
            "noun" => Self::Noun,
            "adverb" => Self::Adverb,
            "adjective" => Self::Adjective,
            "pronounce" => Self::Pronounce,
            "determiner" => Self::Determiner,
            _ => Self::Undefined(value.to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WordUsageExample(pub String);
