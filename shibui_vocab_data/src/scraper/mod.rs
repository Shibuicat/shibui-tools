pub mod cambridge_dictionary_scraper;
pub use cambridge_dictionary_scraper::CambridgeDictionaryScraper;
use serde::Serialize;
pub mod tratu_coviet;
pub mod oxford_scraper;
pub mod page_scraper;
pub mod wiktionary_scraper;
pub use oxford_scraper::OxfordScraper;
pub use wiktionary_scraper::WiktionaryScraper;

use std::future::Future;

use anyhow::Result;

pub trait Scraper {
    fn fetch(&self, word: &str) -> impl Future<Output = Result<Option<WordDefinition>>>;
}

#[derive(Debug, Serialize)]
pub struct WordDefinition {
    pub word: String,
    pub classes: Vec<WordClass>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<Section>,
    #[serde(skip_serializing)]
    pub extracted_html: Option<String>,
}

impl WordDefinition {
    pub fn has_meanings(&self) -> bool {
        self.classes
            .iter()
            .flat_map(|class| class.definitions.iter())
            .flat_map(|definition| definition.contexts.iter())
            .any(|context| !context.meanings.is_empty())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Section {
    pub key: Option<String>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    pub items: Vec<SectionItem>,
}

#[derive(Debug, Serialize)]
pub struct SectionItem {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WordPronounce {
    pub region: Region,
    pub ipa: String,
    pub link: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub enum Region {
    US,
    UK,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
    Exclamation,
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
            "exclamation" => Self::Exclamation,
            _ => Self::Undefined(value.to_owned()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WordUsageExample(pub String);
