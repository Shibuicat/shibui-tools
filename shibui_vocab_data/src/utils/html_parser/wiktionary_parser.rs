use anyhow::bail;
use scraper::{ElementRef, Html, Selector};

use super::html_dom::{
    child_elements, child_sections, first_descendant, has_class, heading_of, select_first, text_of,
};
use super::wiktionary_fragment::clean_html;
use super::wiktionary_pronunciation::pronounces_of;
use super::wiktionary_senses::definition_of;
use super::{HtmlParser, WordNotFoundError};
use crate::scraper::{
    Class, Group, Section, SectionItem, WordClass, WordDefinition, WordForm, WordPronounce,
};

const PARTS_OF_SPEECH: [&str; 19] = [
    "noun", "verb", "adjective", "adverb", "pronoun", "preposition", "conjunction", "interjection",
    "determiner", "numeral", "proverb", "phrase", "prefix", "suffix", "particle", "article",
    "contraction", "symbol", "letter",
];

const SKIPPED_HEADINGS: [&str; 10] = [
    "references", "further reading", "descendants", "translations", "quotations", "gallery",
    "conjugation", "declension", "inflection", "statistics",
];

const SECTION_KEYS: [(&str, &str); 11] = [
    ("synonyms", "synonym"),
    ("antonyms", "antonym"),
    ("derived terms", "derived"),
    ("related terms", "related"),
    ("hyponyms", "hyponym"),
    ("hypernyms", "hypernym"),
    ("coordinate terms", "coordinate"),
    ("alternative forms", "alternative-form"),
    ("see also", "see-also"),
    ("anagrams", "anagram"),
    ("usage notes", "usage-note"),
];

#[derive(Clone)]
pub struct WiktionaryHtmlParser;

#[derive(Default)]
struct Collected {
    classes: Vec<WordClass>,
    sections: Vec<Section>,
    etymologies: Vec<(String, Collected)>,
}

impl Collected {
    fn into_group(self, number: usize, label: String) -> Group {
        Group {
            key: format!("etymology-{number}"),
            kind: "etymology".to_owned(),
            label,
            attribution: None,
            frequency: None,
            pronunciations: Vec::new(),
            classes: self.classes,
            sections: self.sections,
        }
    }
}

impl HtmlParser for WiktionaryHtmlParser {
    type Output = WordDefinition;

    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        let document = Html::parse_document(html_content);
        let english = match english_section(&document) {
            Some(english) => english,
            None => bail!(WordNotFoundError),
        };

        let mut collected = Collected::default();
        let split = child_sections(english).filter(|section| is_etymology(*section)).count() > 1;
        collect(english, &[], &mut collected, split);

        let mut etymologies = collected.etymologies.into_iter();
        let (mut classes, mut sections) = (collected.classes, collected.sections);
        if let Some((_, first)) = etymologies.next() {
            classes.extend(first.classes);
            sections.extend(first.sections);
        }
        let groups = etymologies
            .enumerate()
            .map(|(index, (label, part))| part.into_group(index + 2, label))
            .collect();

        let definition = WordDefinition {
            groups,
            extracted_html: Some(clean_html(english)),
            ..WordDefinition::new(headword_of(english).unwrap_or_default(), classes, sections)
        };
        if !definition.has_meanings() {
            bail!(WordNotFoundError);
        }
        Ok(definition)
    }
}

fn english_section(document: &Html) -> Option<ElementRef> {
    let selector = Selector::parse("section").unwrap();
    document.select(&selector).find(|section| {
        heading_of(*section).is_some_and(|heading| heading.value().attr("id") == Some("English"))
    })
}

fn headword_of(english: ElementRef) -> Option<String> {
    first_descendant(english, &|element| {
        element.value().name() == "strong" && element.value().classes().any(|class| class == "headword")
    })
    .map(text_of)
}

fn heading_text(section: ElementRef) -> String {
    heading_of(section).map(text_of).unwrap_or_default()
}

fn is_etymology(section: ElementRef) -> bool {
    heading_text(section).to_lowercase().starts_with("etymology")
}

fn collect(scope: ElementRef, inherited: &[WordPronounce], collected: &mut Collected, split: bool) {
    let pronounces = child_sections(scope)
        .find(|section| heading_text(*section).eq_ignore_ascii_case("pronunciation"))
        .map(pronounces_of)
        .unwrap_or_else(|| inherited.to_vec());

    child_sections(scope).for_each(|section| {
        let label = heading_text(section);
        let lowered = label.to_lowercase();

        if lowered.starts_with("etymology") && split {
            let mut part = Collected::default();
            part.sections.extend(etymology_section(section, &label));
            collect(section, &pronounces, &mut part, false);
            collected.etymologies.push((label, part));
        } else if lowered.starts_with("etymology") {
            collected.sections.extend(etymology_section(section, &label));
            collect(section, &pronounces, collected, false);
        } else if PARTS_OF_SPEECH.contains(&lowered.as_str()) {
            let mut class = WordClass::new(
                Class::from(lowered.as_str()),
                vec![definition_of(section)],
                pronounces.clone(),
            );
            apply_headword_line(section, &mut class);
            collected.classes.push(class);
            child_sections(section).for_each(|nested| {
                collected.sections.extend(list_section(nested, Some(&lowered)));
            });
        } else if lowered != "pronunciation" {
            collected.sections.extend(list_section(section, None));
        }
    });
}

fn apply_headword_line(section: ElementRef, class: &mut WordClass) {
    let Some(line) = select_first(section, "span.headword-line") else {
        return;
    };
    let mut label: Option<String> = None;
    child_elements(line).for_each(|part| match part.value().name() {
        "i" if select_first(part, "a[href*=\"Glossary\"]").is_some() => {
            class.grammar = Some(text_of(part)).filter(|text| !text.is_empty());
        }
        "i" => {
            let text = text_of(part);
            if !matches!(text.as_str(), "or" | "and" | "") {
                label = Some(text);
            }
        }
        "b" if has_class(part, "form-of") => class.forms.push(WordForm {
            label: label.clone(),
            text: text_of(part),
            audio: None,
            pronunciations: Vec::new(),
        }),
        _ => {}
    });
}

fn etymology_section(section: ElementRef, label: &str) -> Option<Section> {
    let items = child_elements(section)
        .filter(|child| child.value().name() == "p")
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(|text| SectionItem::new(text, None))
        .collect::<Vec<_>>();
    (!items.is_empty()).then(|| Section {
        key: Some("etymology".to_owned()),
        label: label.to_owned(),
        class_name: None,
        items,
    })
}

fn list_section(section: ElementRef, class_name: Option<&str>) -> Option<Section> {
    let label = heading_text(section);
    let lowered = label.to_lowercase();
    if SKIPPED_HEADINGS.contains(&lowered.as_str()) || PARTS_OF_SPEECH.contains(&lowered.as_str()) {
        return None;
    }

    let items = items_of(section);
    (!items.is_empty()).then(|| Section {
        key: SECTION_KEYS
            .iter()
            .find(|(heading, _)| *heading == lowered)
            .map(|(_, key)| (*key).to_owned()),
        label,
        class_name: class_name.map(str::to_owned),
        items,
    })
}

fn items_of(section: ElementRef) -> Vec<SectionItem> {
    let list_items = section
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "li" && owner_section(*element) == Some(section))
        .flat_map(items_of_line)
        .collect::<Vec<_>>();
    if !list_items.is_empty() {
        return list_items;
    }

    child_elements(section)
        .filter(|child| child.value().name() == "p")
        .map(item_of)
        .collect()
}

fn owner_section(element: ElementRef) -> Option<ElementRef> {
    element
        .ancestors()
        .filter_map(ElementRef::wrap)
        .find(|ancestor| ancestor.value().name() == "section")
}

fn items_of_line(line: ElementRef) -> Vec<SectionItem> {
    let words = line
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "span" && element.value().attr("lang").is_some())
        .filter_map(|span| first_descendant(span, &|candidate| candidate.value().name() == "a"))
        .map(item_of)
        .collect::<Vec<_>>();
    if words.is_empty() {
        vec![item_of(line)]
    } else {
        words
    }
}

fn item_of(element: ElementRef) -> SectionItem {
    let link = Some(element)
        .filter(|candidate| candidate.value().name() == "a")
        .or_else(|| first_descendant(element, &|candidate| candidate.value().name() == "a"))
        .and_then(|anchor| anchor.value().attr("href"))
        .map(link_target);
    SectionItem::new(text_of(element), link)
}

fn link_target(href: &str) -> String {
    href.trim_start_matches("./").split('#').next().unwrap_or_default().replace('_', " ")
}
