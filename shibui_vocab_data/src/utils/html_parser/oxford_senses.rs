use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, text_of};
use crate::scraper::{ClassDefinition, WordContext, WordExplanation, WordUsageExample};

const DESCRIPTION_CLASSES: [&str; 5] = ["grammar", "labels", "dis-g", "use", "cf"];

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
    let description = group
        .into_iter()
        .chain(
            parts
                .iter()
                .filter(|part| DESCRIPTION_CLASSES.iter().any(|class| has_class(**part, class)))
                .map(|part| text_of(*part)),
        )
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let examples = parts
        .iter()
        .filter(|part| has_class(**part, "examples"))
        .flat_map(|list| child_elements(*list))
        .filter_map(|item| child_elements(item).find(|child| has_class(*child, "x")))
        .map(|example| WordUsageExample(text_of(example)))
        .collect();

    Some(WordContext {
        description: (!description.is_empty()).then_some(description),
        meanings: vec![WordExplanation { explanation, examples }],
    })
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
