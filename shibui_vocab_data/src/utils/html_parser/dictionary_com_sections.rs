use scraper::ElementRef;

use super::html_dom::{select_all, select_first, text_of};
use crate::scraper::{LinkKind, Section, SectionItem, WordUsageExample};

pub fn sections_of(root: ElementRef) -> Vec<Section> {
    std::iter::once(etymology(root))
        .chain(notes(root))
        .chain([
        paragraphs(root, "explanation", "Explanation", "section#id-sec-explanation div.box-explanation-text p"),
        word_forms(root),
        links(root, "related", "Related Words", "section#id-sec-related-words a.lk-related-word"),
        examples(root),
    ])
    .flatten()
    .collect()
}

fn section(key: &str, label: &str, items: Vec<SectionItem>) -> Option<Section> {
    (!items.is_empty()).then(|| Section {
        key: Some(key.to_owned()),
        label: label.to_owned(),
        class_name: None,
        items,
    })
}

pub fn idioms_of(group: ElementRef) -> Option<Section> {
    let label = select_first(group, "ol.list-definition h3").map(text_of).unwrap_or_else(|| "Idioms".to_owned());
    let items = select_all(group, "li.list-more-idioms-item a").into_iter().filter_map(word_link).collect();
    section("idioms", &label, items)
}

fn etymology(root: ElementRef) -> Option<Section> {
    let items = select_all(root, "section#id-sec-history-origin p")
        .into_iter()
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(|text| SectionItem::new(text, None))
        .collect();
    section("etymology", "Etymology", items)
}

fn notes(root: ElementRef) -> Vec<Option<Section>> {
    select_all(root, "section.sec-list-notes")
        .into_iter()
        .map(|block| {
            let label = select_first(block, "h2").map(text_of).unwrap_or_else(|| "Notes".to_owned());
            let items = select_all(block, "p.txt-item-notes, li")
                .into_iter()
                .map(text_of)
                .filter(|text| !text.is_empty())
                .map(|text| SectionItem::new(text, None))
                .collect();
            let key = block
                .value()
                .attr("id")
                .map(|id| id.trim_start_matches("id-sec-").to_owned())
                .unwrap_or_else(|| "notes".to_owned());
            section(&key, &label, items)
        })
        .collect()
}

fn paragraphs(root: ElementRef, key: &str, label: &str, css: &str) -> Option<Section> {
    let items = select_all(root, css)
        .into_iter()
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(|text| SectionItem::new(text, None))
        .collect();
    section(key, label, items)
}

fn links(root: ElementRef, key: &str, label: &str, css: &str) -> Option<Section> {
    let items = select_all(root, css).into_iter().filter_map(word_link).collect();
    section(key, label, items)
}

fn word_link(anchor: ElementRef) -> Option<SectionItem> {
    let text = select_first(anchor, "div")
        .map(text_of)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| text_of(anchor));
    let href = anchor.value().attr("href")?;
    (!text.is_empty()).then(|| SectionItem {
        link_kind: Some(LinkKind::Word),
        label: select_first(anchor, "div.other-word-form-pos").map(text_of).filter(|pos| !pos.is_empty()),
        ..SectionItem::new(text, Some(href.trim_start_matches("/browse/").to_owned()))
    })
}

fn word_forms(root: ElementRef) -> Option<Section> {
    let items = select_all(root, "section#id-sec-other-word-forms a.other-word-form")
        .into_iter()
        .filter_map(word_link)
        .collect();
    section("word-forms", "Other Word Forms", items)
}

fn examples(root: ElementRef) -> Option<Section> {
    let items = select_all(root, "section#id-sec-example-sentences blockquote.box-examples")
        .into_iter()
        .filter_map(|block| {
            let text = select_first(block, "p.txt-example").map(text_of).filter(|text| !text.is_empty())?;
            let source = select_first(block, "cite").map(text_of).filter(|source| !source.is_empty());
            let date = select_first(block, "p.txt-source")
                .map(|line| text_of(line))
                .and_then(|line| line.rsplit_once('●').map(|(_, date)| date.trim().to_owned()))
                .filter(|date| !date.is_empty());
            Some(SectionItem {
                examples: vec![WordUsageExample {
                    text: text.clone(),
                    source: source.map(|title| crate::scraper::ExampleSource {
                        title: Some(title),
                        date,
                        link: select_first(block, "a.lk-ex-source").and_then(|a| a.value().attr("href")).map(str::to_owned),
                        ..Default::default()
                    }),
                    ..WordUsageExample::default()
                }],
                ..SectionItem::new(text, None)
            })
        })
        .collect();
    section("examples", "Example Sentences", items)
}
