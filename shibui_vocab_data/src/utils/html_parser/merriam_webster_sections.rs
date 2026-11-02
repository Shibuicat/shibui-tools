use scraper::ElementRef;

use super::html_dom::{select_all, text_of};
use crate::scraper::{Section, SectionItem};

pub fn sections_of(root: ElementRef) -> Vec<Section> {
    [
        ("etymology", "Word History", "div#word-history p.et"),
        ("synonyms", "Synonyms", "div#synonyms li.dictionary-synonyms-item"),
        ("phrases", "Phrases containing the word", "div#related-phrases a"),
        ("rhymes", "Rhymes", "div#related-rhymes a"),
        ("run-on", "Run-on entries", "div.uro"),
    ]
    .into_iter()
    .filter_map(|(key, label, css)| section_of(root, key, label, css))
    .collect()
}

fn section_of(root: ElementRef, key: &str, label: &str, css: &str) -> Option<Section> {
    let items = select_all(root, css)
        .into_iter()
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(|text| SectionItem::new(text, None))
        .collect::<Vec<_>>();
    (!items.is_empty()).then(|| Section {
        key: Some(key.to_owned()),
        label: label.to_owned(),
        class_name: None,
        items,
    })
}
