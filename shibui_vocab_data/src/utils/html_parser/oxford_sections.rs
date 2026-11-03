use scraper::ElementRef;

use super::html_dom::{
    child_elements, has_ancestor_class, has_class, path_of, select_all, select_first, text_of, text_without,
};
use crate::scraper::{RelationTerm, Section, SectionItem, WordUsageExample};

const WORD_LISTS: [(&str, &str); 4] = [
    ("list=ox3000", "Oxford 3000"),
    ("list=ox5000", "Oxford 5000"),
    ("list=opal_written", "OPAL Written"),
    ("list=opal_spoken", "OPAL Spoken"),
];

const GENERIC_BOXES: [(&str, &str, &str); 8] = [
    ("synonyms", "synonym-usage", "Synonym Usage"),
    ("langbank", "language-bank", "Language Bank"),
    ("mlt", "more-like-this", "More Like This"),
    ("express", "express-yourself", "Express Yourself"),
    ("which_word", "which-word", "Which Word?"),
    ("homophone", "homophone", "Homophones"),
    ("vocab", "vocabulary-building", "Vocabulary Building"),
    ("british_american", "british-american", "British/American"),
];

#[derive(Default)]
struct SectionBook {
    sections: Vec<Section>,
}

impl SectionBook {
    fn add(&mut self, key: &str, label: &str, items: Vec<SectionItem>) {
        let existing = self
            .sections
            .iter_mut()
            .find(|section| section.key.as_deref() == Some(key) && section.label == label);
        match existing {
            Some(section) => extend_unique(&mut section.items, items),
            None => {
                let mut unique = vec![];
                extend_unique(&mut unique, items);
                if !unique.is_empty() {
                    self.sections.push(Section {
                        key: Some(key.to_owned()),
                        label: label.to_owned(),
                        class_name: None,
                        items: unique,
                    });
                }
            }
        }
    }
}

fn extend_unique(into: &mut Vec<SectionItem>, items: Vec<SectionItem>) {
    items.into_iter().filter(|item| !item.text.is_empty()).for_each(|item| {
        if !into.iter().any(|existing| existing.text == item.text && existing.description == item.description) {
            into.push(item);
        }
    });
}

fn plain(text: String) -> SectionItem {
    SectionItem::new(text, None)
}

fn outside_idioms(element: &ElementRef) -> bool {
    !has_ancestor_class(*element, "idioms")
}

pub fn sections_of(entry: ElementRef) -> Vec<Section> {
    let mut book = SectionBook::default();
    levels(entry, &mut book);
    relations(entry, &mut book);
    topics(entry, &mut book);
    forms(entry, &mut book);
    boxes(entry, &mut book);
    idioms(entry, &mut book);
    phrasal_verbs(entry, &mut book);
    book.sections
}

fn levels(entry: ElementRef, book: &mut SectionBook) {
    let lists = select_all(entry, "div.symbols a")
        .into_iter()
        .filter_map(|anchor| anchor.value().attr("href"))
        .flat_map(|href| WORD_LISTS.iter().filter(move |(marker, _)| href.contains(marker)))
        .map(|(_, name)| plain((*name).to_owned()));
    book.add("word-list", "Word lists", lists.collect());
}

pub fn relation_kind(prefix: &str) -> (&'static str, &'static str) {
    match prefix.to_lowercase().as_str() {
        "synonym" => ("synonym", "Synonyms"),
        "opposite" => ("antonym", "Antonyms"),
        _ => ("see-also", "See also"),
    }
}

fn relations(entry: ElementRef, book: &mut SectionBook) {
    select_all(entry, "span.xrefs")
        .into_iter()
        .filter(outside_idioms)
        .filter(|xrefs| !has_ancestor_class(*xrefs, "sense"))
        .for_each(|xrefs| {
            let prefix = select_first(xrefs, "span.prefix").map(text_of).unwrap_or_default();
            let (key, label) = relation_kind(&prefix);
            book.add(key, label, references_in(xrefs));
        });
}

pub fn terms_in(scope: ElementRef) -> Vec<RelationTerm> {
    references_in(scope)
        .into_iter()
        .map(|item| RelationTerm { text: item.text, link: item.link, relevance: None, examples: Vec::new() })
        .collect()
}

fn references_in(scope: ElementRef) -> Vec<SectionItem> {
    select_all(scope, "a.Ref")
        .into_iter()
        .map(|anchor| {
            SectionItem::new(
                select_first(anchor, "span.xh").or(Some(anchor)).map(text_of).unwrap_or_default(),
                anchor.value().attr("href").map(word_link),
            )
        })
        .collect()
}

fn word_link(href: &str) -> String {
    let last = href.split('#').next().unwrap_or_default().rsplit('/').next().unwrap_or_default();
    let base = match last.rsplit_once('_') {
        Some((word, number)) if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) => word,
        _ => last,
    };
    base.replace('-', " ")
}

fn topics(entry: ElementRef, book: &mut SectionBook) {
    let items = select_all(entry, "span.topic-g a.Ref")
        .into_iter()
        .filter(outside_idioms)
        .map(|anchor| {
            let name = select_first(anchor, "span.topic_name").map(text_of).unwrap_or_default();
            let level = select_first(anchor, "span.topic_cefr").map(text_of);
            SectionItem::new(
                match level {
                    Some(level) => format!("{name} ({level})"),
                    None => name,
                },
                anchor.value().attr("href").map(path_of),
            )
        })
        .collect();
    book.add("topic", "Topics", items);
}

fn forms(entry: ElementRef, book: &mut SectionBook) {
    let variants = select_all(entry, "div.variants span.v-g").into_iter().map(|variant| plain(text_of(variant)));
    book.add("alternative-form", "Alternative forms", variants.collect());

    let inflections = select_all(entry, "div.inflections")
        .into_iter()
        .flat_map(|inflections| {
            text_of(inflections)
                .trim_matches(|c| c == '(' || c == ')')
                .split(", ")
                .map(|part| plain(part.to_owned()))
                .collect::<Vec<_>>()
        });
    book.add("inflection", "Inflections", inflections.collect());
}

fn boxes(entry: ElementRef, book: &mut SectionBook) {
    select_all(entry, "span.unbox[unbox]").into_iter().for_each(|unbox| {
        let kind = unbox.value().attr("unbox").unwrap_or_default();
        match kind {
            "verbforms" => {}
            "wordorigin" => book.add("etymology", "Word Origin", paragraphs(unbox)),
            "snippet" | "colloc" => book.add("collocation", "Collocations", collocations(unbox)),
            "extra_examples" => book.add("extra-example", "Extra Examples", extra_examples(unbox)),
            "wordfamily" => book.add("word-family", "Word Family", list_items(unbox)),
            other => match GENERIC_BOXES.iter().find(|(attribute, _, _)| *attribute == other) {
                Some((_, key, label)) => book.add(key, label, vec![plain(body_text(unbox))]),
                None => book.add(&other.replace('_', "-"), &box_title(unbox, other), vec![plain(body_text(unbox))]),
            },
        }
    });
}

fn box_title(unbox: ElementRef, fallback: &str) -> String {
    select_first(unbox, "span.box_title")
        .map(|title| text_without(title, &|part| has_class(part, "closed")))
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| fallback.to_owned())
}

fn list_items(unbox: ElementRef) -> Vec<SectionItem> {
    select_all(unbox, "li").into_iter().map(|item| plain(text_of(item))).collect()
}

fn paragraphs(unbox: ElementRef) -> Vec<SectionItem> {
    let items = select_all(unbox, "span.p").into_iter().map(|paragraph| plain(text_of(paragraph))).collect::<Vec<_>>();
    if items.is_empty() {
        vec![plain(body_text(unbox))]
    } else {
        items
    }
}

fn collocations(unbox: ElementRef) -> Vec<SectionItem> {
    select_all(unbox, "li")
        .into_iter()
        .map(|item| text_of(item))
        .filter(|text| text != "…")
        .map(plain)
        .collect()
}

fn extra_examples(unbox: ElementRef) -> Vec<SectionItem> {
    select_all(unbox, "span.unx").into_iter().map(|example| plain(text_of(example))).collect()
}

fn body_text(unbox: ElementRef) -> String {
    child_elements(unbox)
        .find(|child| has_class(*child, "body"))
        .map(|body| child_elements(body).map(text_of).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

fn idioms(entry: ElementRef, book: &mut SectionBook) {
    let items = select_all(entry, "div.idioms span.idm-g")
        .into_iter()
        .flat_map(|idiom| {
            let phrase = select_first(idiom, "span.idm").map(text_of).unwrap_or_default();
            select_all(idiom, "li.sense")
                .into_iter()
                .filter_map(|sense| {
                    let definition = select_first(sense, "span.def").map(text_of)?;
                    Some(SectionItem {
                        description: Some(definition),
                        examples: select_all(sense, "span.x")
                            .into_iter()
                            .map(|example| text_without(example, &|part| has_class(part, "gloss")))
                            .filter(|text| !text.is_empty())
                            .map(WordUsageExample::plain)
                            .collect(),
                        ..SectionItem::new(phrase.clone(), None)
                    })
                })
                .collect::<Vec<_>>()
        });
    book.add("idiom", "Idioms", items.collect());
}

fn phrasal_verbs(entry: ElementRef, book: &mut SectionBook) {
    let items = select_all(entry, "aside.phrasal_verb_links a.Ref").into_iter().map(|anchor| {
        SectionItem::new(
            select_first(anchor, "span.xh").map(text_of).unwrap_or_else(|| text_of(anchor)),
            anchor.value().attr("href").map(word_link),
        )
    });
    book.add("phrasal-verb", "Phrasal Verbs", items.collect());
}
