use anyhow::{anyhow, bail};
use scraper::Html;

use super::html_dom::{child_elements, has_class, select_all, select_first, text_of, text_without};
use super::oxford_fragment::clean_entry;
use super::oxford_pronunciation::pronounces_of;
use super::oxford_sections::sections_of;
use super::oxford_senses::definition_of;
use super::{HtmlParser, WordNotFoundError};
use crate::scraper::{Class, WordClass, WordDefinition, WordForm};

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
        let mut class = WordClass::new(
            Class::from(class_name.to_lowercase().as_str()),
            vec![definition_of(entry)],
            pronounces_of(webtop),
        );
        class.level = headword_level(webtop);
        class.forms = forms_of(entry);
        let definition = WordDefinition {
            extracted_html: Some(clean_entry(content)),
            ..WordDefinition::new(
                select_first(webtop, "h1.headword")
                    .map(|headword| text_without(headword, &|part| has_class(part, "hm")))
                    .unwrap_or_default(),
                vec![class],
                sections_of(entry),
            )
        };
        if !definition.has_meanings() {
            let root = document.root_element();
            bail!(if select_first(root, "li.sense").is_some() || select_first(root, "aside.phrasal_verb_links").is_some() {
                anyhow::Error::new(WordNotFoundError)
            } else {
                layout_changed()
            });
        }
        Ok(definition)
    }
}

fn headword_level(webtop: scraper::ElementRef) -> Option<String> {
    child_elements(webtop)
        .filter(|part| has_class(*part, "symbols"))
        .flat_map(|symbols| select_all(symbols, "span"))
        .find_map(|symbol| {
            symbol
                .value()
                .classes()
                .find_map(|class| class.strip_prefix("ox3ksym_").or_else(|| class.strip_prefix("ox5ksym_")))
                .map(str::to_uppercase)
        })
}

fn forms_of(entry: scraper::ElementRef) -> Vec<WordForm> {
    select_all(entry, "tr.verb_form")
        .into_iter()
        .filter_map(|row| {
            let cell = select_first(row, "td.verb_form")?;
            let text = text_without(cell, &|part| has_class(part, "vf_prefix"));
            (!text.is_empty()).then(|| WordForm {
                label: select_first(cell, "span.vf_prefix").map(text_of).filter(|label| !label.is_empty()),
                text,
                audio: None,
                pronunciations: select_first(row, "td.verb_phons").map(pronounces_of).unwrap_or_default(),
            })
        })
        .collect()
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

