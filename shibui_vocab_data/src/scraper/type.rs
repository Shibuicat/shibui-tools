use std::future::Future;

use anyhow::Result;

pub trait Scraper {
    fn fetch(&self, word: &str) -> impl Future<Output = Result<Option<WordDefinition>>>;
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
    pub contexts: Vec<WordContext>,
}

#[derive(Debug)]
pub struct WordContext {
    pub description: Option<String>,
    pub meanings: Vec<WordExplanation>,
}

#[derive(Debug)]
pub struct WordExplanation {
    pub explanation: String,
    pub examples: Vec<WordUsageExample>,
}

#[derive(Debug)]
pub enum Class {
    Noun,
    Verb,
    Adverb,
    Adjective,
    Pronounce,
    Determiner,
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

#[derive(Debug)]
pub struct WordUsageExample(pub String);
