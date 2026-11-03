use scraper::ElementRef;

use super::cambridge_senses::{example_of, has_own_meanings, meaning_of};
use super::cambridge_terms::{anchor_link, link_of, non_empty_text, xref_kind};
use super::html_dom::{has_ancestor_class, has_class, select_all, select_first, text_of, text_without};
use crate::scraper::{Section, SectionItem, WordUsageExample};

pub fn sections_of(entry: ElementRef, class_name: &str) -> Vec<Section> {
    let scoped = references(entry)
        .into_iter()
        .chain(phrases(entry))
        .chain(derived_forms(entry))
        .chain(vocabulary(entry))
        .chain(topics(entry))
        .map(|section| Section { class_name: Some(class_name.to_owned()), ..section })
        .collect::<Vec<_>>();
    merge(scoped)
}

pub fn merge(sections: Vec<Section>) -> Vec<Section> {
    sections.into_iter().fold(Vec::new(), |mut merged: Vec<Section>, section| {
        match merged
            .iter_mut()
            .find(|known| known.key == section.key && known.class_name == section.class_name)
        {
            Some(known) => known.items.extend(section.items),
            None => merged.push(section),
        }
        merged
    })
}

fn section(key: &str, label: String, items: Vec<SectionItem>) -> Option<Section> {
    (!items.is_empty()).then(|| Section { key: Some(key.to_owned()), label, class_name: None, items })
}

fn references(entry: ElementRef) -> Vec<Section> {
    select_all(entry, "div.xref")
        .into_iter()
        .filter(|xref| !has_ancestor_class(*xref, "def-block"))
        .filter_map(|xref| {
            let kind = xref_kind(xref)?;
            let label = non_empty_text(select_first(xref, ".xref-title")).unwrap_or_else(|| kind.clone());
            let items = select_all(xref, "div.item").into_iter().filter_map(reference_of).collect();
            section(&kind, label, items)
        })
        .collect()
}

fn reference_of(item: ElementRef) -> Option<SectionItem> {
    let text = non_empty_text(select_first(item, ".x-h"))?;
    let (link, link_kind) = anchor_link(item, &text);
    Some(SectionItem {
        link,
        link_kind,
        description: non_empty_text(select_first(item, ".x-pos")),
        ..SectionItem::new(text, None)
    })
}

fn phrases(entry: ElementRef) -> Option<Section> {
    if !has_own_meanings(entry) {
        return None;
    }
    let items = select_all(entry, "div.phrase-block").into_iter().flat_map(phrase_items).collect();
    section("phrase", "Phrases and idioms".to_owned(), items)
}

fn phrase_items(block: ElementRef) -> Vec<SectionItem> {
    let Some(title) = non_empty_text(select_first(block, "span.phrase-title")) else {
        return Vec::new();
    };
    let shared = select_all(block, "span.phrase-info span.usage").into_iter().map(text_of);
    let shared = shared.collect::<Vec<_>>();
    select_all(block, "div.def-block")
        .into_iter()
        .filter_map(meaning_of)
        .map(|meaning| {
            let labels = shared
                .iter()
                .cloned()
                .chain(meaning.labels.iter().map(|label| label.text.clone()))
                .collect::<Vec<_>>()
                .join(", ");
            SectionItem {
                label: Some(labels).filter(|labels| !labels.is_empty()),
                description: Some(meaning.explanation),
                examples: meaning.examples,
                ..SectionItem::new(title.clone(), None)
            }
        })
        .collect()
}

fn derived_forms(entry: ElementRef) -> Option<Section> {
    let items = select_all(entry, "div.runon").into_iter().filter_map(derived_form).collect();
    section("derived", "Derived forms".to_owned(), items)
}

fn derived_form(runon: ElementRef) -> Option<SectionItem> {
    let text = non_empty_text(select_first(runon, "h3.runon-title"))?;
    Some(SectionItem {
        label: non_empty_text(select_first(runon, "span.pos")),
        description: non_empty_text(select_first(runon, "div.def")),
        examples: select_all(runon, "div.examp").into_iter().filter_map(example_of).collect(),
        audio: select_first(runon, "audio source")
            .and_then(|source| source.value().attr("src"))
            .map(str::to_owned),
        ..SectionItem::new(text, None)
    })
}

fn vocabulary(entry: ElementRef) -> Vec<Section> {
    select_all(entry, "div.daccord")
        .into_iter()
        .filter(|accordion| select_first(*accordion, "div.daccord_lt").is_some())
        .filter_map(|accordion| {
            let (key, label) = if has_class(accordion, "smartt") {
                ("vocabulary", "SMART Vocabulary")
            } else {
                ("thesaurus", "Thesaurus")
            };
            section(key, label.to_owned(), accordion_items(accordion))
        })
        .collect()
}

fn topics(entry: ElementRef) -> Option<Section> {
    let items = select_all(entry, "div.smartt a.tb")
        .into_iter()
        .filter_map(|anchor| {
            let text = non_empty_text(Some(anchor))?;
            let (link, link_kind) = anchor.value().attr("href").map(|href| link_of(href, &text)).unwrap_or((None, None));
            Some(SectionItem { link, link_kind, ..SectionItem::new(text, None) })
        })
        .collect();
    section("topic", "Topics".to_owned(), items)
}

fn accordion_items(accordion: ElementRef) -> Vec<SectionItem> {
    let parts = select_all(accordion, "div.daccord_lt, div.daccord_lb");
    let (_, items) = parts.into_iter().fold((None, Vec::new()), |(topic, mut items), part| {
        if has_class(part, "daccord_lt") {
            (non_empty_text(Some(part)), items)
        } else {
            items.extend(select_all(part, "li").into_iter().filter_map(|li| vocabulary_item(li, topic.clone())));
            (topic, items)
        }
    });
    items
}

fn vocabulary_item(li: ElementRef, topic: Option<String>) -> Option<SectionItem> {
    let text = non_empty_text(select_first(li, ".haf")).or_else(|| non_empty_text(select_first(li, "a")))?;
    let (link, link_kind) = select_first(li, "a")
        .and_then(|anchor| anchor.value().attr("href"))
        .map(|href| link_of(href, &text))
        .unwrap_or((None, None));
    Some(SectionItem {
        link,
        link_kind,
        label: topic,
        description: non_empty_text(select_first(li, "span.pos")),
        examples: select_all(li, ".dexample")
            .into_iter()
            .map(|example| text_without(example, &|part| has_class(part, "lab")))
            .filter(|example| !example.is_empty())
            .map(WordUsageExample::plain)
            .collect(),
        ..SectionItem::new(text, None)
    })
}
