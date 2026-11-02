use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, select_all, select_first, text_of, text_without};
use crate::scraper::{Section, SectionItem, WordUsageExample};

pub fn sections_of(block: ElementRef) -> Vec<Section> {
    [derived_forms(block), etymology(block), notes(block)].into_iter().flatten().collect()
}

fn section(key: &str, label: &str, items: Vec<SectionItem>) -> Option<Section> {
    (!items.is_empty()).then(|| Section {
        key: Some(key.to_owned()),
        label: label.to_owned(),
        class_name: None,
        items,
    })
}

fn derived_forms(block: ElementRef) -> Option<Section> {
    let items = select_all(block, "div.hom_subsec span.form.type-drv")
        .into_iter()
        .filter_map(|form| {
            let text = select_first(form, "span.orth").map(text_of).filter(|text| !text.is_empty())?;
            let label = form
                .next_siblings()
                .filter_map(ElementRef::wrap)
                .find(|sibling| has_class(*sibling, "hom"))
                .and_then(|hom| select_first(hom, "span.pos"))
                .map(text_of)
                .filter(|pos| !pos.is_empty());
            Some(SectionItem { label, ..SectionItem::new(text, None) })
        })
        .chain(select_all(block, "div.re.sense").into_iter().filter_map(run_on))
        .collect();
    section("derived", "Derived forms", items)
}

fn run_on(entry: ElementRef) -> Option<SectionItem> {
    let text = select_first(entry, "span.form span.orth").map(text_of).filter(|text| !text.is_empty())?;
    Some(SectionItem {
        label: select_first(entry, "span.pos").map(text_of).filter(|pos| !pos.is_empty()),
        examples: select_all(entry, "div.cit.type-example span.quote")
            .into_iter()
            .map(text_of)
            .filter(|example| !example.is_empty())
            .map(WordUsageExample::plain)
            .collect(),
        audio: select_first(entry, "a.hwd_sound")
            .and_then(|link| link.value().attr("data-src-mp3"))
            .map(str::to_owned),
        ..SectionItem::new(text, None)
    })
}

fn notes(block: ElementRef) -> Option<Section> {
    let items = select_first(block, "div.content.definitions")
        .into_iter()
        .flat_map(|content| {
            child_elements(content)
                .flat_map(|child| if has_class(child, "hom") { child_elements(child).collect::<Vec<_>>() } else { vec![child] })
                .collect::<Vec<_>>()
        })
        .filter(|child| has_class(*child, "note"))
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(|text| SectionItem::new(text, None))
        .collect();
    section("note", "Notes", items)
}

fn etymology(block: ElementRef) -> Option<Section> {
    let items = child_elements(select_first(block, "div.dictlink > div")?)
        .filter(|child| has_class(*child, "etyms"))
        .map(|etym| text_without(etym, &|part| has_class(part, "entry_title")))
        .filter(|text| !text.is_empty())
        .map(|text| SectionItem::new(text, None))
        .collect();
    section("etymology", "Word origin", items)
}
