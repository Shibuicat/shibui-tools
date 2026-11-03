use anyhow::bail;
use scraper::{ElementRef, Html};

use super::cambridge_sections::{merge, sections_of};
use super::cambridge_senses::definitions_of;
use super::cambridge_terms::non_empty_text;
use super::html_dom::{has_class, select_all, select_first, text_of, text_without};
use super::{HtmlParser, WordNotFoundError};
use crate::scraper::{Class, Group, Region, WordClass, WordDefinition, WordForm, WordPronounce};

const DEFAULT_DICTIONARY: &str = "cald4";
const UNKNOWN_CLASS: &str = "unknown";

#[derive(Clone)]
pub struct CambridgeHtmlParser;

impl HtmlParser for CambridgeHtmlParser {
    type Output = WordDefinition;

    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        let document = Html::parse_document(html_content);
        let entries = entries_of(document.root_element());
        let mut dictionaries = by_dictionary(&entries);
        if dictionaries.is_empty() {
            bail!(WordNotFoundError);
        }
        let primary_at = dictionaries.iter().position(|(key, _)| key == DEFAULT_DICTIONARY).unwrap_or(0);
        let (_, primary) = dictionaries.remove(primary_at);
        let word = word_of(primary[0]);
        let (classes, sections) = block_of(&primary, &word);

        let definition = WordDefinition {
            groups: dictionaries.iter().map(|(key, entries)| group_of(key, entries, &word)).collect(),
            extracted_html: Some(entries.iter().map(|entry| entry.html()).collect::<Vec<_>>().join("\n")),
            ..WordDefinition::new(word, classes, sections)
        };
        if !definition.has_meanings() {
            bail!(WordNotFoundError);
        }
        Ok(definition)
    }
}

fn entries_of(root: ElementRef) -> Vec<ElementRef> {
    let standard = select_all(root, "div.pr.entry-body__el");
    let entries = if standard.is_empty() { select_all(root, "div.entry-body__el") } else { standard };
    if entries.is_empty() {
        select_all(root, "div.pr.idiom-block")
            .into_iter()
            .filter(|block| select_first(*block, "div.def").is_some())
            .collect()
    } else {
        entries
    }
}

fn by_dictionary<'a>(entries: &[ElementRef<'a>]) -> Vec<(String, Vec<ElementRef<'a>>)> {
    entries.iter().fold(Vec::new(), |mut grouped: Vec<(String, Vec<ElementRef<'a>>)>, entry| {
        let key = dictionary_of(*entry);
        match grouped.iter_mut().find(|(known, _)| *known == key) {
            Some((_, members)) => members.push(*entry),
            None => grouped.push((key, vec![*entry])),
        }
        grouped
    })
}

fn dictionary_of(entry: ElementRef) -> String {
    select_first(entry, "div.cid")
        .and_then(|marker| marker.value().attr("id"))
        .and_then(|id| id.split('-').next())
        .filter(|key| !key.is_empty())
        .unwrap_or(DEFAULT_DICTIONARY)
        .to_owned()
}

fn dictionary_label(key: &str) -> String {
    match key {
        "cald4" => "English".to_owned(),
        "cacd" => "American English".to_owned(),
        "cbed" => "Business English".to_owned(),
        other => other.to_uppercase(),
    }
}

fn group_of(key: &str, entries: &[ElementRef], word: &str) -> Group {
    let (classes, sections) = block_of(entries, word);
    Group {
        key: key.to_owned(),
        kind: "dictionary".to_owned(),
        label: dictionary_label(key),
        attribution: None,
        frequency: None,
        pronunciations: Vec::new(),
        classes,
        sections,
    }
}

fn block_of(entries: &[ElementRef], word: &str) -> (Vec<WordClass>, Vec<crate::scraper::Section>) {
    let classes = entries
        .iter()
        .map(|entry| class_of(*entry, word))
        .filter(|class| class.definitions.iter().any(|definition| !definition.contexts.is_empty()))
        .collect();
    let sections = entries
        .iter()
        .flat_map(|entry| sections_of(*entry, &class_name_of(*entry).to_lowercase()))
        .collect();
    (classes, merge(sections))
}

fn word_of(entry: ElementRef) -> String {
    non_empty_text(select_first(entry, "span.hw"))
        .or_else(|| title_of(entry))
        .unwrap_or_default()
}

fn title_of(entry: ElementRef) -> Option<String> {
    non_empty_text(select_first(entry, "div.di-title"))
}

fn class_name_of(entry: ElementRef) -> String {
    non_empty_text(select_first(entry, "span.pos")).unwrap_or_else(|| UNKNOWN_CLASS.to_owned())
}

fn class_of(entry: ElementRef, word: &str) -> WordClass {
    let header = select_first(entry, "div.pos-header");
    let mut class = WordClass::new(
        Class::from(class_name_of(entry).as_str()),
        definitions_of(entry),
        header.map(pronunciations_of).unwrap_or_default(),
    );
    class.grammar = non_empty_text(header.and_then(|header| select_first(header, "div.posgram span.gram")));
    class.forms = header.map(forms_of).unwrap_or_default();
    class.sub_class = title_of(entry)
        .filter(|title| !title.eq_ignore_ascii_case(word))
        .or_else(|| header.and_then(shared_parts_of_speech));
    class
}

fn shared_parts_of_speech(header: ElementRef) -> Option<String> {
    let posgram = select_first(header, "div.posgram")?;
    (select_all(posgram, "span.pos").len() > 1)
        .then(|| text_without(posgram, &|part| has_class(part, "gram")))
        .filter(|text| !text.is_empty())
}

fn pronunciations_of(header: ElementRef) -> Vec<WordPronounce> {
    [(Region::UK, "span.uk.dpron-i"), (Region::US, "span.us.dpron-i")]
        .into_iter()
        .flat_map(|(region, selector)| {
            select_all(header, selector).into_iter().map(move |block| pronunciation_of(region.clone(), block))
        })
        .filter(|pronounce| !pronounce.ipa.is_empty() || !pronounce.link.is_empty())
        .fold(Vec::new(), |mut unique: Vec<WordPronounce>, pronounce| {
            let seen = unique.iter().any(|known| {
                known.region == pronounce.region && known.ipa == pronounce.ipa && known.link == pronounce.link
            });
            if !seen {
                unique.push(pronounce);
            }
            unique
        })
}

fn pronunciation_of(region: Region, block: ElementRef) -> WordPronounce {
    WordPronounce {
        region,
        ipa: select_first(block, "span.ipa").map(text_of).unwrap_or_default(),
        link: select_first(block, "audio source")
            .and_then(|source| source.value().attr("src"))
            .unwrap_or_default()
            .to_owned(),
    }
}

fn forms_of(header: ElementRef) -> Vec<WordForm> {
    let inflections = select_all(header, "span.inf-group").into_iter().flat_map(inflection_forms);
    let variants = select_all(header, "span.var, span.spellvar").into_iter().flat_map(variant_forms);
    inflections.chain(variants).collect()
}

fn inflection_forms(group: ElementRef) -> Vec<WordForm> {
    let (_, forms) = group.descendants().filter_map(ElementRef::wrap).fold(
        (None, Vec::new()),
        |(label, mut forms), element| {
            if has_class(element, "lab") {
                (non_empty_text(Some(element)), forms)
            } else if has_class(element, "inf") {
                forms.push(WordForm {
                    label: label.clone(),
                    text: text_of(element),
                    audio: None,
                    pronunciations: Vec::new(),
                });
                (label, forms)
            } else {
                (label, forms)
            }
        },
    );
    forms.into_iter().filter(|form| !form.text.is_empty()).collect()
}

fn variant_forms(variant: ElementRef) -> Vec<WordForm> {
    let label = non_empty_text(select_first(variant, "span.lab"));
    select_all(variant, "span.v")
        .into_iter()
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(|text| WordForm { label: label.clone(), text, audio: None, pronunciations: Vec::new() })
        .collect()
}
