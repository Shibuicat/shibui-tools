use anyhow::{anyhow, bail};
use scraper::{ElementRef, Html};

use super::collins_fragment::clean_entries;
use super::collins_sections::sections_of;
use super::collins_senses::{contexts_of, definition_of};
use super::html_dom::{child_elements, has_class, select_all, select_first, text_of, text_without};
use super::{HtmlParser, WordNotFoundError};
use crate::scraper::{Class, Group, Region, WordClass, WordDefinition, WordForm, WordPronounce};

const NOT_FOUND_TEXT: &str = "Sorry, no results for";
const DICTIONARY_MARKERS: [&str; 4] = ["dictionary", "dictentry", "dictionaries", "dictlink"];
const UNKNOWN_CLASS: &str = "unknown";

#[derive(Clone)]
pub struct CollinsHtmlParser;

impl HtmlParser for CollinsHtmlParser {
    type Output = WordDefinition;

    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        let document = Html::parse_document(html_content);
        let root = document.root_element();
        let blocks = select_all(root, "div.dictentry");
        if blocks.is_empty() {
            bail!(missing_blocks(html_content));
        }

        let word = select_first(blocks[0], "h2.h2_entry span.orth").map(text_of).ok_or_else(|| missing_headword(root))?;
        let mut parsed = unique_keys(blocks.iter().map(|block| parsed_block(*block)).filter(Parsed::has_content).collect());
        if parsed.is_empty() {
            bail!(layout_changed());
        }
        let primary = parsed.remove(0);

        let definition = WordDefinition {
            groups: parsed.into_iter().map(Parsed::into_group).collect(),
            extracted_html: Some(clean_entries(root)),
            ..WordDefinition::new(word, primary.classes, primary.sections)
        };
        if !definition.has_meanings() {
            bail!(layout_changed());
        }
        Ok(definition)
    }
}

fn unique_keys(parsed: Vec<Parsed>) -> Vec<Parsed> {
    let totals = parsed.iter().fold(std::collections::HashMap::new(), |mut counts, entry| {
        *counts.entry(entry.key.clone()).or_insert(0usize) += 1;
        counts
    });
    let mut seen = std::collections::HashMap::new();
    parsed
        .into_iter()
        .map(|mut entry| {
            if totals[&entry.key] > 1 {
                let number = seen.entry(entry.key.clone()).or_insert(0usize);
                *number += 1;
                entry.key = format!("{}-{number}", entry.key);
            }
            entry
        })
        .collect()
}

struct Parsed {
    key: String,
    label: String,
    attribution: Option<String>,
    frequency: Option<u32>,
    pronunciations: Vec<WordPronounce>,
    classes: Vec<WordClass>,
    sections: Vec<crate::scraper::Section>,
}

impl Parsed {
    fn has_content(&self) -> bool {
        !self.classes.is_empty() || !self.sections.is_empty()
    }

    fn into_group(self) -> Group {
        Group {
            key: self.key,
            kind: "dictionary".to_owned(),
            label: self.label,
            attribution: self.attribution,
            frequency: self.frequency,
            pronunciations: self.pronunciations,
            classes: self.classes,
            sections: self.sections,
        }
    }
}

fn parsed_block(block: ElementRef) -> Parsed {
    let key = dictionary_key(block);
    let copyright = select_first(block, "div.copyright");
    let pronunciations = pronunciations_of(block);
    let definitions = select_first(block, "div.content.definitions");
    let mut classes = definitions.map(|content| classes_of(content, &pronunciations)).unwrap_or_default();
    if let (Some(first), Some(content)) = (classes.first_mut(), definitions) {
        first.forms = forms_of(content);
    }

    Parsed {
        label: copyright
            .and_then(|c| select_first(c, "span.i"))
            .map(text_of)
            .filter(|text| !text.is_empty())
            .or_else(|| select_first(block, "span.dictname").map(text_of).filter(|text| !text.is_empty()))
            .unwrap_or_else(|| key.clone()),
        attribution: copyright.map(text_of).filter(|text| !text.is_empty()),
        frequency: select_first(block, "span.word-frequency-img")
            .and_then(|element| element.value().attr("data-band"))
            .and_then(|band| band.parse().ok()),
        pronunciations,
        classes,
        sections: sections_of(block),
        key,
    }
}

fn dictionary_key(entry: ElementRef) -> String {
    let own = entry.value().classes().find(|class| !DICTIONARY_MARKERS.contains(class));
    let from_parent = || {
        entry
            .parent()
            .and_then(ElementRef::wrap)
            .and_then(|parent| parent.value().classes().find(|class| !DICTIONARY_MARKERS.contains(class)))
    };
    own.or_else(from_parent).unwrap_or("collins").to_owned()
}

fn classes_of(content: ElementRef, pronunciations: &[WordPronounce]) -> Vec<WordClass> {
    child_elements(content)
        .filter(|child| has_class(*child, "hom"))
        .map(|hom| class_of(hom, pronunciations))
        .fold(Vec::new(), merge_class)
        .into_iter()
        .filter(|class| class.definitions.iter().any(|definition| !definition.contexts.is_empty()))
        .collect()
}

fn merge_class(mut classes: Vec<WordClass>, next: WordClass) -> Vec<WordClass> {
    match classes.last_mut() {
        Some(last) if same_class(last, &next) => {
            let contexts = next.definitions.into_iter().flat_map(|definition| definition.contexts);
            if let Some(definition) = last.definitions.first_mut() {
                definition.contexts.extend(contexts);
            }
        }
        _ => classes.push(next),
    }
    classes
}

fn same_class(left: &WordClass, right: &WordClass) -> bool {
    let name_of = |class: &WordClass| serde_json::to_value(&class.class_name).ok();
    name_of(right) == Some(serde_json::Value::String(UNKNOWN_CLASS.to_owned()))
        || (name_of(left) == name_of(right) && left.sub_class == right.sub_class)
}

fn class_of(hom: ElementRef, pronunciations: &[WordPronounce]) -> WordClass {
    let grammar = child_elements(hom).find(|child| has_class(*child, "gramGrp") || has_class(*child, "pos"));
    let name = grammar
        .and_then(|element| if has_class(element, "pos") { Some(element) } else { select_first(element, "span.pos") })
        .map(text_of)
        .map(|text| text.to_lowercase())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| UNKNOWN_CLASS.to_owned());
    let mut class = WordClass::new(
        Class::from(name.as_str()),
        vec![definition_of(contexts_of(hom))],
        pronunciations.to_vec(),
    );
    class.sub_class = grammar.and_then(|element| select_first(element, "span.subc")).map(text_of).filter(|t| !t.is_empty());
    class
}

fn forms_of(content: ElementRef) -> Vec<WordForm> {
    let Some(inflected) = child_elements(content).find(|child| has_class(*child, "inflected_forms")) else {
        return Vec::new();
    };
    let mut pending: Vec<String> = Vec::new();
    let mut forms = Vec::new();
    child_elements(inflected).for_each(|part| {
        if has_class(part, "lbl") {
            let label = text_of(part).trim_matches(|c: char| c == ',' || c.is_whitespace()).to_owned();
            if !label.is_empty() {
                pending.push(label);
            }
        } else if has_class(part, "orth") {
            forms.push(WordForm {
                label: (!pending.is_empty()).then(|| pending.join(", ")),
                text: text_of(part),
                audio: None,
                pronunciations: Vec::new(),
            });
            pending.clear();
        } else if has_class(part, "hwd_sound") {
            if let Some(form) = forms.last_mut() {
                form.audio = audio_of(part);
            }
        }
    });
    forms.retain(|form| !form.text.is_empty());
    forms
}

fn pronunciations_of(block: ElementRef) -> Vec<WordPronounce> {
    let Some(pron) = select_first(block, "div.mini_h2 span.pron") else {
        return Vec::new();
    };
    let ipa = text_without(pron, &|part| has_class(part, "ptr") || has_class(part, "punctuation"));
    select_all(pron, "a.hwd_sound")
        .into_iter()
        .filter_map(|link| {
            let region = match link.value().attr("data-lang")? {
                "en_GB" => Region::UK,
                "en_US" => Region::US,
                _ => return None,
            };
            Some(WordPronounce { region, ipa: ipa.clone(), link: link.value().attr("data-src-mp3")?.to_owned() })
        })
        .collect()
}

fn audio_of(sound: ElementRef) -> Option<String> {
    select_first(sound, "a.hwd_sound")
        .and_then(|link| link.value().attr("data-src-mp3"))
        .map(str::to_owned)
}

fn missing_blocks(html: &str) -> anyhow::Error {
    if html.contains(NOT_FOUND_TEXT) {
        anyhow::Error::new(WordNotFoundError)
    } else {
        layout_changed()
    }
}

fn missing_headword(root: ElementRef) -> anyhow::Error {
    if select_first(root, "h2.h2_entry").is_none() {
        anyhow::Error::new(WordNotFoundError)
    } else {
        layout_changed()
    }
}

fn layout_changed() -> anyhow::Error {
    anyhow!("Collins page has no readable entry; the layout may have changed")
}
