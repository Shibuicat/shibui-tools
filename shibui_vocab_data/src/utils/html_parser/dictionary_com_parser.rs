use anyhow::{anyhow, bail};
use scraper::{ElementRef, Html};

use super::dictionary_com_fragment::clean_entries;
use super::dictionary_com_senses::definition_of;
use super::dictionary_com_sections::{idioms_of, sections_of};
use super::html_dom::{select_all, select_first, text_of};
use super::{HtmlParser, WordNotFoundError};
use crate::scraper::{Class, Group, Region, WordClass, WordDefinition, WordForm, WordPronounce};

const GROUP_ID_PREFIX: &str = "id-sec-entry-group-";
const PRIMARY_GROUP: &str = "dcom";
const UNKNOWN_CLASS: &str = "unknown";

#[derive(Clone)]
pub struct DictionaryComHtmlParser;

impl HtmlParser for DictionaryComHtmlParser {
    type Output = WordDefinition;

    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        let document = Html::parse_document(html_content);
        let root = document.root_element();
        let mut groups = select_all(root, "section.sec-item-entry-group")
            .into_iter()
            .map(group_of)
            .collect::<Vec<_>>();
        let Some(primary_at) = groups.iter().position(|group| group.key == PRIMARY_GROUP) else {
            bail!(missing_entries(root));
        };
        let primary = groups.remove(primary_at);
        let word = select_first(root, "h1.hdr-headword").map(text_of).ok_or_else(layout_changed)?;

        let definition = WordDefinition {
            groups,
            extracted_html: Some(clean_entries(root)),
            ..WordDefinition::new(word, primary.classes, sections_of(root))
        };
        if !definition.has_meanings() {
            bail!(layout_changed());
        }
        Ok(definition)
    }
}

fn group_of(section: ElementRef) -> Group {
    let key = section
        .value()
        .attr("id")
        .map(|id| id.trim_start_matches(GROUP_ID_PREFIX).to_owned())
        .unwrap_or_default();
    let label = select_first(section, "em.txt-entry-group-type").map(text_of).unwrap_or_default();
    let (kind, label) = match key.as_str() {
        "dcom" | "british" => ("region", label),
        "idiom" => ("idioms", "Idioms".to_owned()),
        _ => ("domain", label),
    };
    Group {
        label: if label.is_empty() { key.clone() } else { label },
        kind: kind.to_owned(),
        key,
        attribution: select_first(section, "cite").map(text_of).filter(|text| !text.is_empty()),
        frequency: None,
        pronunciations: Vec::new(),
        classes: select_all(section, "div.box-item-entry")
            .into_iter()
            .flat_map(classes_of)
            .filter(|class| class.definitions.iter().any(|definition| !definition.contexts.is_empty()))
            .collect(),
        sections: idioms_of(section).into_iter().collect(),
    }
}

fn classes_of(entry: ElementRef) -> Vec<WordClass> {
    let pronunciations = pronunciations_of(entry);
    select_all(entry, "div.box-posb")
        .into_iter()
        .map(|posb| class_of(posb, &pronunciations))
        .collect()
}

fn class_of(posb: ElementRef, pronunciations: &[WordPronounce]) -> WordClass {
    let label = select_first(posb, "div.box-pos-label h2").map(text_of).unwrap_or_default();
    let (name, detail) = split_label(&label);
    let mut class = WordClass::new(
        Class::from(name.as_str()),
        vec![definition_of(posb)],
        pronunciations.to_vec(),
    );
    class.sub_class = detail;
    class.forms = select_all(posb, "li.item-inflected-form-and-tooltip").into_iter().filter_map(form_of).collect();
    class
}

fn split_label(label: &str) -> (String, Option<String>) {
    let lowered = label.to_lowercase();
    match lowered.split_once('(') {
        Some((name, detail)) => (
            name.trim().to_owned(),
            Some(detail.trim_end_matches(')').trim().to_owned()).filter(|detail| !detail.is_empty()),
        ),
        None if lowered.is_empty() => (UNKNOWN_CLASS.to_owned(), None),
        None => (lowered.trim().to_owned(), None),
    }
}

fn form_of(item: ElementRef) -> Option<WordForm> {
    let text = select_first(item, "span.inflected-form-form").map(text_of).filter(|text| !text.is_empty())?;
    Some(WordForm {
        label: select_first(item, "span.tooltip-content").map(text_of).filter(|label| !label.is_empty()),
        text,
        audio: None,
        pronunciations: Vec::new(),
    })
}

fn pronunciations_of(entry: ElementRef) -> Vec<WordPronounce> {
    let region = match entry.ancestors().filter_map(ElementRef::wrap).find_map(|ancestor| ancestor.value().attr("id")) {
        Some(id) if id.ends_with("british") => Region::UK,
        Some(id) if id.ends_with(PRIMARY_GROUP) => Region::US,
        _ => return Vec::new(),
    };
    let Some(audio) = select_first(entry, "div.box-audio-pronunciation") else {
        return Vec::new();
    };
    let link = select_first(audio, "button[data-audiosrc]").and_then(|button| {
        let origin = button.value().attr("data-audioorigin")?;
        let source = button.value().attr("data-audiosrc")?;
        Some(format!("{}/{}", origin.trim_end_matches('/'), source.trim_start_matches('/')))
    });
    let ipa = select_first(audio, "span.txt-ipa")
        .map(text_of)
        .map(|text| text.trim_matches('/').trim().to_owned())
        .unwrap_or_default();
    match link {
        Some(link) => vec![WordPronounce { region, ipa, link }],
        None => Vec::new(),
    }
}

fn missing_entries(root: ElementRef) -> anyhow::Error {
    let title = select_first(root, "title").map(text_of).unwrap_or_default().to_lowercase();
    let names_a_word = select_first(root, "h1.hdr-headword").is_some();
    if title.contains("not found") || title.contains("404") || names_a_word {
        anyhow::Error::new(WordNotFoundError)
    } else {
        layout_changed()
    }
}

fn layout_changed() -> anyhow::Error {
    anyhow!("Dictionary.com page has no readable entry; the layout may have changed")
}
