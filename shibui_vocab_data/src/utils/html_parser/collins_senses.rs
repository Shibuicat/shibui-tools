use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, select_all, select_first, text_of, text_without};
use crate::scraper::{
    ClassDefinition, Label, Relation, RelationTerm, WordContext, WordExplanation, WordUsageExample,
};

pub fn contexts_of(hom: ElementRef) -> Vec<WordContext> {
    let grammar = child_elements(hom).find(|child| has_class(*child, "gramGrp"));
    let level = grammar
        .and_then(|element| select_first(element, "a[data-type]"))
        .map(text_of)
        .filter(|text| !text.is_empty());
    let pattern = grammar
        .map(|element| select_all(element, "span.lbl.type-syntax"))
        .map(|labels| labels.into_iter().map(text_of).collect::<Vec<_>>().join(" "))
        .filter(|text| !text.is_empty());
    let inherited = child_elements(hom).filter_map(label_of).collect::<Vec<_>>();

    let has_senses = child_elements(hom).any(|child| has_class(child, "sense") && !has_class(child, "re"));
    let own_definition = (!has_senses && child_elements(hom).any(|child| has_class(child, "def")))
        .then(|| meanings_of(hom, &[]))
        .unwrap_or_default();

    child_elements(hom)
        .filter(|child| (has_class(*child, "sense") && !has_class(*child, "re")) || is_cross_reference(*child))
        .flat_map(|sense| meanings_of(sense, &inherited))
        .chain(own_definition)
        .map(|mut meaning| {
            meaning.level = level.clone();
            meaning.pattern = meaning.pattern.or_else(|| pattern.clone());
            meaning
        })
        .map(|meaning| WordContext { description: None, meanings: vec![meaning] })
        .collect()
}

fn is_cross_reference(element: ElementRef) -> bool {
    has_class(element, "xr") && element.value().name() == "a"
}

pub fn definition_of(contexts: Vec<WordContext>) -> ClassDefinition {
    ClassDefinition { contexts }
}

fn meanings_of(sense: ElementRef, inherited: &[Label]) -> Vec<WordExplanation> {
    let labels = inherited.iter().cloned().chain(child_elements(sense).filter_map(label_of)).collect::<Vec<_>>();
    let nested = child_elements(sense)
        .filter(|child| has_class(*child, "sense") && !has_class(*child, "re"))
        .collect::<Vec<_>>();
    let Some(explanation) = explanation_of(sense) else {
        return nested.into_iter().flat_map(|sub| meanings_of(sub, &labels)).collect();
    };

    let mut meaning = WordExplanation::new(explanation, examples_of(sense));
    meaning.labels = labels;
    meaning.sub_meanings = nested.into_iter().flat_map(|sub| meanings_of(sub, &[])).collect();
    meaning.grammar = child_elements(sense)
        .find(|child| has_class(*child, "subc"))
        .map(text_of)
        .filter(|text| !text.is_empty());
    meaning.notes = child_elements(sense)
        .filter(|child| has_class(*child, "note") || (has_class(*child, "re") && !has_class(*child, "sense")))
        .map(|note| text_of(note).trim_start_matches('▶').trim().to_owned())
        .filter(|text| !text.is_empty())
        .collect();
    meaning.relations = child_elements(sense).filter(|child| has_class(*child, "thes")).filter_map(synonyms_of).collect();
    vec![meaning]
}

fn explanation_of(sense: ElementRef) -> Option<String> {
    if has_class(sense, "xr") {
        return Some(text_of(sense)).filter(|text| !text.is_empty());
    }
    if has_class(sense, "def") {
        return Some(text_without(sense, &|part| has_class(part, "sensenum") || has_class(part, "lbl")))
            .filter(|text| !text.is_empty());
    }
    child_elements(sense)
        .find(|child| has_class(*child, "def"))
        .or_else(|| child_elements(sense).find(|child| has_class(*child, "xr")))
        .map(text_of)
        .map(|text| text.trim_start_matches(|c: char| matches!(c, '.' | ':' | ';' | ',') || c.is_whitespace()).to_owned())
        .filter(|text| !text.is_empty())
}

fn label_of(element: ElementRef) -> Option<Label> {
    if element.value().name() != "span" || !has_class(element, "lbl") || has_class(element, "type-syntax") {
        return None;
    }
    let kind = element
        .value()
        .classes()
        .find_map(|class| class.strip_prefix("type-"))
        .map(|kind| match kind {
            "subj" => "subject",
            "geo" => "region",
            "mod" => "modifier",
            other => other,
        })
        .unwrap_or("usage");
    let text = text_of(element).trim_matches(|c: char| c == '[' || c == ']' || c.is_whitespace()).to_owned();
    (!text.is_empty()).then(|| Label { kind: kind.to_owned(), text })
}

fn examples_of(sense: ElementRef) -> Vec<WordUsageExample> {
    child_elements(sense)
        .filter(|child| has_class(*child, "cit") && has_class(*child, "type-example"))
        .filter_map(|example| {
            let text = text_without(example, &|part| {
                has_class(part, "lbl") || has_class(part, "ptr") || has_class(part, "cit")
            });
            (!text.is_empty()).then(|| WordUsageExample {
                pattern: select_all(example, ".lbl.type-syntax")
                    .into_iter()
                    .next()
                    .map(text_of)
                    .filter(|pattern| !pattern.is_empty()),
                ..WordUsageExample::plain(text)
            })
        })
        .collect()
}

fn synonyms_of(thes: ElementRef) -> Option<Relation> {
    let terms = child_elements(thes)
        .filter(|child| !has_class(*child, "type-thesaurus") && matches!(child.value().name(), "a" | "span"))
        .filter(|child| has_class(*child, "form"))
        .map(|term| {
            let anchor = if term.value().name() == "a" { Some(term) } else { select_first(term, "a") };
            RelationTerm {
                text: anchor
                    .map(text_of)
                    .unwrap_or_else(|| text_without(term, &|part| has_class(part, "lbl"))),
                link: anchor.and_then(|a| a.value().attr("href")).map(str::to_owned),
                relevance: None,
                examples: Vec::new(),
            }
        })
        .filter(|term| !term.text.is_empty())
        .collect::<Vec<_>>();
    (!terms.is_empty()).then(|| Relation { kind: "synonym".to_owned(), terms })
}
