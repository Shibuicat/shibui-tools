use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, text_of};
use super::html_dom::{select_first, text_without};
use crate::scraper::{ClassDefinition, Label, WordContext, WordExplanation, WordUsageExample};

pub fn definition_of(entry: ElementRef) -> ClassDefinition {
    ClassDefinition {
        contexts: child_elements(entry)
            .filter(|child| has_class(*child, "senses_multiple") || has_class(*child, "sense_single"))
            .flat_map(grouped_senses)
            .filter_map(|(group, sense)| context_of(group, sense))
            .collect(),
    }
}

fn grouped_senses(list: ElementRef) -> Vec<(Option<String>, ElementRef)> {
    child_elements(list)
        .flat_map(|child| {
            if has_class(child, "shcut-g") {
                let heading = child_elements(child).find(|part| has_class(*part, "shcut")).map(text_of);
                child_elements(child)
                    .filter(|part| has_class(*part, "sense"))
                    .map(|sense| (heading.clone(), sense))
                    .collect::<Vec<_>>()
            } else if has_class(child, "sense") {
                vec![(None, child)]
            } else {
                vec![]
            }
        })
        .collect()
}

fn context_of(group: Option<String>, sense: ElementRef) -> Option<WordContext> {
    let parts = parts_of(sense);
    let explanation = parts.iter().find(|part| has_class(**part, "def")).map(|def| text_of(*def))?;
    let examples = parts
        .iter()
        .filter(|part| has_class(**part, "examples"))
        .flat_map(|list| child_elements(*list))
        .filter_map(example_of)
        .collect();

    let mut meaning = WordExplanation::new(explanation, examples);
    meaning.level = sense.value().attr("cefr").map(str::to_uppercase);
    meaning.grammar = text_of_class(&parts, "grammar").map(|text| trim_marks(&text));
    meaning.pattern = text_of_class(&parts, "cf");
    meaning.labels = labels_of(&parts);
    meaning.notes = parts.iter().filter(|part| has_class(**part, "use")).map(|part| text_of(*part)).collect();

    Some(WordContext {
        description: group.filter(|text| !text.is_empty()),
        meanings: vec![meaning],
    })
}

fn example_of(item: ElementRef) -> Option<WordUsageExample> {
    let example = child_elements(item).find(|child| has_class(*child, "x"))?;
    Some(WordUsageExample {
        text: text_without(example, &|part| has_class(part, "gloss")),
        pattern: child_elements(item).find(|child| has_class(*child, "cf")).map(text_of),
        gloss: select_first(example, "span.gloss").map(text_of),
        ..WordUsageExample::default()
    })
}

fn labels_of(parts: &[ElementRef]) -> Vec<Label> {
    [("labels", "usage"), ("dis-g", "domain")]
        .into_iter()
        .flat_map(|(class, kind)| {
            parts
                .iter()
                .filter(move |part| has_class(**part, class))
                .map(move |part| (kind, trim_marks(&text_of(*part))))
        })
        .filter(|(_, text)| !text.is_empty())
        .map(|(kind, text)| Label { kind: kind.to_owned(), text })
        .collect()
}

fn text_of_class(parts: &[ElementRef], class: &str) -> Option<String> {
    parts
        .iter()
        .find(|part| has_class(**part, class))
        .map(|part| text_of(*part))
        .filter(|text| !text.is_empty())
}

fn trim_marks(text: &str) -> String {
    text.trim_matches(|c: char| matches!(c, '[' | ']' | '(' | ')') || c.is_whitespace()).to_owned()
}

fn parts_of(sense: ElementRef) -> Vec<ElementRef> {
    let direct = child_elements(sense).collect::<Vec<_>>();
    let nested = direct
        .iter()
        .filter(|child| has_class(**child, "sensetop"))
        .flat_map(|top| child_elements(*top))
        .collect::<Vec<_>>();
    direct.into_iter().chain(nested).collect()
}
