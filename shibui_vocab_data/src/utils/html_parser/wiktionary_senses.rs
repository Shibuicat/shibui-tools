use scraper::ElementRef;

use super::wiktionary_dom::{child_elements, has_class, text_without};
use crate::scraper::{ClassDefinition, WordContext, WordExplanation, WordUsageExample};

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
    WordExplanation {
        explanation: explanation_of(sense),
        examples: examples_of(sense),
    }
}

fn is_not_definition_text(element: ElementRef) -> bool {
    matches!(element.value().name(), "ul" | "ol" | "dl" | "sup") || has_class(element, "defdate")
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
        .map(|example| WordUsageExample(text_without(example, &|_| false)))
        .filter(|example| !example.0.is_empty())
        .collect()
}
