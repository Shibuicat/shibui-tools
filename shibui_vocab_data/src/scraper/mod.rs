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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<Group>,
    #[serde(skip_serializing)]
    pub extracted_html: Option<String>,
}

impl WordDefinition {
    pub fn new(word: String, classes: Vec<WordClass>, sections: Vec<Section>) -> Self {
        Self {
            word,
            classes,
            sections,
            groups: Vec::new(),
            extracted_html: None,
        }
    }

    pub fn has_meanings(&self) -> bool {
        self.classes
            .iter()
            .chain(self.groups.iter().flat_map(|group| group.classes.iter()))
            .flat_map(|class| class.definitions.iter())
            .flat_map(|definition| definition.contexts.iter())
            .any(|context| !context.meanings.is_empty())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub key: String,
    pub kind: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pronunciations: Vec<WordPronounce>,
    pub classes: Vec<WordClass>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<Section>,
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

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SectionItem {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_kind: Option<LinkKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<WordUsageExample>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
}

impl SectionItem {
    pub fn new(text: String, link: Option<String>) -> Self {
        Self {
            text,
            link,
            ..Self::default()
        }
    }
}

#[derive(Debug, Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LinkKind {
    Word,
    Path,
    Audio,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    pub definitions: Vec<ClassDefinition>,
    pub pronunciations: Vec<WordPronounce>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub forms: Vec<WordForm>,
}

impl WordClass {
    pub fn new(
        class_name: Class,
        definitions: Vec<ClassDefinition>,
        pronunciations: Vec<WordPronounce>,
    ) -> Self {
        Self {
            class_name,
            sub_class: None,
            grammar: None,
            level: None,
            definitions,
            pronunciations,
            forms: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordForm {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pronunciations: Vec<WordPronounce>,
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
#[serde(rename_all = "camelCase")]
pub struct WordExplanation {
    pub explanation: String,
    pub examples: Vec<WordUsageExample>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sub_meanings: Vec<WordExplanation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grammar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<Topic>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl WordExplanation {
    pub fn new(explanation: String, examples: Vec<WordUsageExample>) -> Self {
        Self {
            explanation,
            examples,
            sub_meanings: Vec::new(),
            labels: Vec::new(),
            grammar: None,
            level: None,
            pattern: None,
            topics: Vec::new(),
            relations: Vec::new(),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Label {
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct Topic {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Relation {
    pub kind: String,
    pub terms: Vec<RelationTerm>,
}

#[derive(Debug, Serialize)]
pub struct RelationTerm {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance: Option<String>,
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

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WordUsageExample {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gloss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<ExampleSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translation: Option<String>,
}

impl WordUsageExample {
    pub fn plain(text: String) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }
}

#[derive(Debug, Serialize, Default)]
pub struct ExampleSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}
