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

    senses.iter().filter(|sense| has_content(**sense)).for_each(|sense| match sub_senses(*sense) {
        Some(sub_senses) => {
            flush(&mut plain, &mut contexts);
            let description = Some(explanation_of(*sense)).filter(|text| !text.is_empty());
            let inherited = if description.is_none() { labels_of(*sense) } else { Vec::new() };
            contexts.push(WordContext {
                description,
                meanings: sub_senses.into_iter().flat_map(|sub| meanings_of(sub, &inherited)).collect(),
            });
        }
        None => plain.extend(meanings_of(*sense, &[])),
    });
    flush(&mut plain, &mut contexts);
    contexts
}

fn meanings_of(sense: ElementRef, inherited: &[Label]) -> Vec<WordExplanation> {
    if !has_content(sense) {
        return Vec::new();
    }
    let labels = inherited.iter().cloned().chain(labels_of(sense)).collect::<Vec<_>>();
    let nested = sub_senses(sense).unwrap_or_default();
    if explanation_of(sense).is_empty() {
        return nested.into_iter().flat_map(|sub| meanings_of(sub, &labels)).collect();
    }
    let mut meaning = WordExplanation::new(explanation_of(sense), examples_of(sense));
    meaning.labels = labels;
    meaning.sub_meanings = nested.into_iter().flat_map(|sub| meanings_of(sub, &[])).collect();
    vec![meaning]
}

fn has_content(sense: ElementRef) -> bool {
    !explanation_of(sense).is_empty() || sub_senses(sense).is_some()
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
        .trim_start_matches(|c: char| c == ':' || c.is_whitespace())
        .to_owned()
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
