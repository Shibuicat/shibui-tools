use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, select_all, select_first, text_of, text_without};
use crate::scraper::{
    ClassDefinition, Label, Relation, RelationTerm, WordContext, WordExplanation, WordUsageExample,
};

pub fn definition_of(posb: ElementRef) -> ClassDefinition {
    ClassDefinition {
        contexts: select_all(posb, "ol.list-definition")
            .into_iter()
            .flat_map(|list| list_items(list, "item-definition"))
            .filter_map(meaning_of)
            .map(|meaning| WordContext { description: None, meanings: vec![meaning] })
            .collect(),
    }
}

fn list_items<'a>(list: ElementRef<'a>, class: &str) -> Vec<ElementRef<'a>> {
    child_elements(list)
        .filter(|child| child.value().name() == "li" && has_class(*child, class))
        .collect()
}

fn meaning_of(item: ElementRef) -> Option<WordExplanation> {
    let (labels, text) = label_and_text(item);
    let sub_meanings = child_elements(item)
        .filter(|child| has_class(*child, "list-sub-definition"))
        .flat_map(|list| list_items(list, "item-sub-definition"))
        .filter_map(meaning_of)
        .collect::<Vec<_>>();

    let heading_only = text.is_empty() && !sub_meanings.is_empty();
    let explanation = if heading_only {
        labels.iter().map(|label| label.text.clone()).collect::<Vec<_>>().join(", ")
    } else {
        text
    };
    if explanation.is_empty() {
        return None;
    }

    let mut meaning = WordExplanation::new(explanation, examples_of(item));
    meaning.labels = if heading_only { Vec::new() } else { labels };
    meaning.sub_meanings = sub_meanings;
    meaning.relations = relations_of(item);
    Some(meaning)
}

fn label_and_text(item: ElementRef) -> (Vec<Label>, String) {
    let Some(paragraph) = child_elements(item).find(|child| has_class(*child, "txt-variant-label-short")) else {
        return (Vec::new(), String::new());
    };
    let labels = child_elements(paragraph)
        .filter(|child| child.value().name() == "em")
        .map(text_of)
        .map(|text| text.trim_end_matches('.').trim().to_owned())
        .filter(|text| !text.is_empty())
        .map(|text| Label { kind: "domain".to_owned(), text })
        .collect();
    let text = text_without(paragraph, &|part| {
        part.value().name() == "em" && part.parent().and_then(ElementRef::wrap) == Some(paragraph)
    });
    (labels, text)
}

fn examples_of(item: ElementRef) -> Vec<WordUsageExample> {
    child_elements(item)
        .filter(|child| has_class(*child, "box-examples"))
        .flat_map(|block| select_all(block, "p.txt-example"))
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(WordUsageExample::plain)
        .collect()
}

fn relations_of(item: ElementRef) -> Vec<Relation> {
    child_elements(item)
        .filter(|child| has_class(*child, "box-synonym-antonym"))
        .filter_map(|block| {
            let kind = select_first(block, "dt.synant-label")
                .map(text_of)
                .map(|text| text.trim_end_matches(':').trim().to_lowercase())
                .map(|text| text.trim_end_matches('s').to_owned())?;
            let terms = select_all(block, "dd a")
                .into_iter()
                .map(|anchor| RelationTerm {
                    text: text_of(anchor),
                    link: anchor.value().attr("href").map(str::to_owned),
                    relevance: None,
                })
                .filter(|term| !term.text.is_empty())
                .collect::<Vec<_>>();
            (!terms.is_empty()).then_some(Relation { kind, terms })
        })
        .collect()
}
