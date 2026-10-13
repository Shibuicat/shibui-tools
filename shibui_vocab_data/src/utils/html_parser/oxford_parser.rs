use anyhow::{anyhow, bail};
use scraper::Html;

use super::html_dom::{child_elements, has_class, select_first, text_of, text_without};
use super::oxford_fragment::clean_entry;
use super::oxford_pronunciation::pronounces_of;
use super::oxford_sections::sections_of;
use super::oxford_senses::definition_of;
use super::{HtmlParser, WordNotFoundError};
use crate::scraper::{Class, WordClass, WordDefinition};

const NOT_FOUND_TITLE: &str = "Did you spell it correctly";
const UNKNOWN_CLASS: &str = "unknown";

#[derive(Clone)]
pub struct OxfordHtmlParser;

impl HtmlParser for OxfordHtmlParser {
    type Output = WordDefinition;

    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        let document = Html::parse_document(html_content);
        let content = match select_first(document.root_element(), "div#entryContent") {
            Some(content) => content,
            None => return Err(missing_content(&document)),
        };
        let entry = select_first(content, "div.entry").ok_or_else(layout_changed)?;
        let webtop = select_first(entry, "div.webtop").ok_or_else(layout_changed)?;

        let class_name = child_elements(webtop)
            .find(|child| has_class(*child, "pos"))
            .map(text_of)
            .unwrap_or_else(|| UNKNOWN_CLASS.to_owned());
        let definition = WordDefinition {
            word: select_first(webtop, "h1.headword")
                .map(|headword| text_without(headword, &|part| has_class(part, "hm")))
                .unwrap_or_default(),
            classes: vec![WordClass {
                class_name: Class::from(class_name.to_lowercase().as_str()),
                definitions: vec![definition_of(entry)],
                pronounces: pronounces_of(webtop),
            }],
            sections: sections_of(entry),
            extracted_html: Some(clean_entry(content)),
        };
        if !definition.has_meanings() {
            bail!(layout_changed());
        }
        Ok(definition)
    }
}

fn missing_content(document: &Html) -> anyhow::Error {
    let title = select_first(document.root_element(), "title").map(text_of).unwrap_or_default();
    if title.contains(NOT_FOUND_TITLE) {
        anyhow::Error::new(WordNotFoundError)
    } else {
        layout_changed()
    }
}

fn layout_changed() -> anyhow::Error {
    anyhow!("Oxford page has no readable entry; the layout may have changed")
}

