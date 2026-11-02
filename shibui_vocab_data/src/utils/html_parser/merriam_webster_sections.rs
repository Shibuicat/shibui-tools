use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, select_all, select_first, text_of};
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
    .chain(first_known_use(root))
    .collect()
}

fn first_known_use(root: ElementRef) -> Option<Section> {
    let mut function_label: Option<String> = None;
    let items = select_first(root, "div.first-known-content-section")
        .into_iter()
        .flat_map(child_elements)
        .filter_map(|part| {
            if has_class(part, "function-label") {
                function_label = Some(text_of(part)).filter(|text| !text.is_empty());
                None
            } else if has_class(part, "ety-sl") {
                Some(SectionItem { label: function_label.clone(), ..SectionItem::new(text_of(part), None) })
            } else {
                None
            }
        })
        .filter(|item| !item.text.is_empty())
        .collect::<Vec<_>>();
    (!items.is_empty()).then(|| Section {
        key: Some("first-known-use".to_owned()),
        label: "First Known Use".to_owned(),
        class_name: None,
        items,
    })
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
