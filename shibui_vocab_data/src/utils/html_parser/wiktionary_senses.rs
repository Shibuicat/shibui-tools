use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, text_without};
use super::html_dom::{select_all, text_of};
use crate::scraper::{ClassDefinition, Label, WordContext, WordExplanation, WordUsageExample};

pub fn definition_of(part_of_speech: ElementRef) -> ClassDefinition {
    let senses = child_elements(part_of_speech)
        .filter(|child| child.value().name() == "ol")
        .flat_map(child_elements)
        .filter(|item| item.value().name() == "li")
        .collect::<Vec<_>>();

    ClassDefinition {
        contexts: contexts_of(&senses),
    }
}

fn contexts_of(senses: &[ElementRef]) -> Vec<WordContext> {
    let mut contexts = Vec::new();
    let mut plain = Vec::new();

    senses.iter().for_each(|sense| match sub_senses(*sense) {
        Some(sub_senses) => {
            flush(&mut plain, &mut contexts);
            contexts.push(WordContext {
                description: Some(explanation_of(*sense)).filter(|text| !text.is_empty()),
                meanings: sub_senses.into_iter().map(meaning_of).collect(),
            });
        }
        None => plain.push(meaning_of(*sense)),
    });
    flush(&mut plain, &mut contexts);
    contexts
}

fn flush(plain: &mut Vec<WordExplanation>, contexts: &mut Vec<WordContext>) {
    if !plain.is_empty() {
        contexts.push(WordContext {
            description: None,
            meanings: std::mem::take(plain),
        });
    }
}

fn sub_senses(sense: ElementRef) -> Option<Vec<ElementRef>> {
    let nested = child_elements(sense)
        .filter(|child| child.value().name() == "ol")
        .flat_map(child_elements)
        .filter(|item| item.value().name() == "li")
        .collect::<Vec<_>>();
    (!nested.is_empty()).then_some(nested)
}

fn meaning_of(sense: ElementRef) -> WordExplanation {
    let mut meaning = WordExplanation::new(explanation_of(sense), examples_of(sense));
    meaning.labels = labels_of(sense);
    meaning
}

fn labels_of(sense: ElementRef) -> Vec<Label> {
    child_elements(sense)
        .filter(|child| has_class(*child, "usage-label-sense"))
        .flat_map(|label| select_all(label, "span.ib-content"))
        .flat_map(|content| text_of(content).split(',').map(|part| part.trim().to_owned()).collect::<Vec<_>>())
        .filter(|text| !text.is_empty())
        .map(|text| Label { kind: "usage".to_owned(), text })
        .collect()
}

fn is_not_definition_text(element: ElementRef) -> bool {
    matches!(element.value().name(), "ul" | "ol" | "dl" | "sup") || has_class(element, "defdate") || has_class(element, "usage-label-sense")
}

fn explanation_of(sense: ElementRef) -> String {
    text_without(sense, &is_not_definition_text)
}

fn examples_of(sense: ElementRef) -> Vec<WordUsageExample> {
    child_elements(sense)
        .filter(|child| child.value().name() == "dl")
        .flat_map(child_elements)
        .filter(|entry| entry.value().name() == "dd")
        .flat_map(|entry| entry.descendants().filter_map(ElementRef::wrap))
        .filter(|candidate| has_class(*candidate, "h-usage-example"))
        .map(|example| WordUsageExample::plain(text_without(example, &|_| false)))
        .filter(|example| !example.text.is_empty())
        .collect()
}
