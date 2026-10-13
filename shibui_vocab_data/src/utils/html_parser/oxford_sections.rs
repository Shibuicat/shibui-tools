use scraper::ElementRef;

use super::html_dom::{
    child_elements, has_ancestor_class, has_class, path_of, select_all, select_first, text_of, text_without,
};
use crate::scraper::{Section, SectionItem};

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
        if !into.iter().any(|existing| existing.text == item.text) {
            into.push(item);
        }
    });
}

fn plain(text: String) -> SectionItem {
    SectionItem { text, link: None }
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
    let headword_levels = select_first(entry, "div.webtop")
        .into_iter()
        .flat_map(child_elements)
        .filter(|part| has_class(*part, "symbols"))
        .flat_map(|symbols| select_all(symbols, "span"))
        .filter_map(|symbol| {
            symbol
                .value()
                .classes()
                .find_map(|class| class.strip_prefix("ox3ksym_"))
                .map(str::to_uppercase)
        })
        .map(|level| plain(format!("Headword: {level}")));
    let sense_levels = select_all(entry, "li.sense")
        .into_iter()
        .filter(outside_idioms)
        .filter_map(|sense| Some((sense.value().attr("sensenum")?, sense.value().attr("cefr")?)))
        .map(|(number, level)| plain(format!("Sense {number}: {}", level.to_uppercase())));
    book.add("level", "CEFR level", headword_levels.chain(sense_levels).collect());

    let lists = select_all(entry, "div.symbols a")
        .into_iter()
        .filter_map(|anchor| anchor.value().attr("href"))
        .filter_map(|href| {
            ["ox3000", "ox5000"]
                .into_iter()
                .find(|list| href.contains(&format!("list={list}")))
        })
        .map(|list| plain(format!("Oxford {}", &list[2..])));
    book.add("word-list", "Word lists", lists.collect());
}

fn relations(entry: ElementRef, book: &mut SectionBook) {
    select_all(entry, "span.xrefs").into_iter().filter(outside_idioms).for_each(|xrefs| {
        let prefix = select_first(xrefs, "span.prefix").map(text_of).unwrap_or_default().to_lowercase();
        let (key, label) = match prefix.as_str() {
            "synonym" => ("synonym", "Synonyms"),
            "opposite" => ("antonym", "Antonyms"),
            _ => ("see-also", "See also"),
        };
        book.add(key, label, references_in(xrefs));
    });
}

fn references_in(scope: ElementRef) -> Vec<SectionItem> {
    select_all(scope, "a.Ref")
        .into_iter()
        .map(|anchor| SectionItem {
            text: select_first(anchor, "span.xh").or(Some(anchor)).map(text_of).unwrap_or_default(),
            link: anchor.value().attr("href").map(word_link),
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
            SectionItem {
                text: match level {
                    Some(level) => format!("{name} ({level})"),
                    None => name,
                },
                link: anchor.value().attr("href").map(path_of),
            }
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
            "verbforms" => book.add("verb-form", "Verb Forms", verb_forms(unbox)),
            "wordorigin" => book.add("etymology", "Word Origin", paragraphs(unbox)),
            "snippet" | "colloc" => book.add("collocation", "Collocations", collocations(unbox)),
            "extra_examples" => book.add("extra-example", "Extra Examples", extra_examples(unbox)),
            other => {
                if let Some((_, key, label)) = GENERIC_BOXES.iter().find(|(attribute, _, _)| *attribute == other) {
                    book.add(key, label, vec![plain(body_text(unbox))]);
                }
            }
        }
    });
}

fn verb_forms(unbox: ElementRef) -> Vec<SectionItem> {
    select_all(unbox, "td.verb_form")
        .into_iter()
        .map(|cell| {
            let prefix = select_first(cell, "span.vf_prefix").map(text_of).unwrap_or_default();
            let form = text_without(cell, &|part| has_class(part, "vf_prefix"));
            plain(format!("{prefix}: {form}"))
        })
        .collect()
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
            select_all(idiom, "span.def")
                .into_iter()
                .map(|definition| plain(format!("{phrase}: {}", text_of(definition))))
                .collect::<Vec<_>>()
        });
    book.add("idiom", "Idioms", items.collect());
}

fn phrasal_verbs(entry: ElementRef, book: &mut SectionBook) {
    let items = select_all(entry, "aside.phrasal_verb_links a.Ref").into_iter().map(|anchor| SectionItem {
        text: select_first(anchor, "span.xh").map(text_of).unwrap_or_else(|| text_of(anchor)),
        link: anchor.value().attr("href").map(word_link),
    });
    book.add("phrasal-verb", "Phrasal Verbs", items.collect());
}
