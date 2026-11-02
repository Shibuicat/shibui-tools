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

    child_elements(hom)
        .filter(|child| has_class(*child, "sense") || (has_class(*child, "xr") && child.value().name() == "a"))
        .filter_map(meaning_of)
        .map(|mut meaning| {
            meaning.level = level.clone();
            meaning.pattern = pattern.clone();
            meaning
        })
        .map(|meaning| WordContext { description: None, meanings: vec![meaning] })
        .collect()
}

pub fn definition_of(contexts: Vec<WordContext>) -> ClassDefinition {
    ClassDefinition { contexts }
}

fn meaning_of(sense: ElementRef) -> Option<WordExplanation> {
    let explanation = explanation_of(sense)?;
    let mut meaning = WordExplanation::new(explanation, examples_of(sense));
    meaning.labels = child_elements(sense).filter_map(label_of).collect();
    meaning.grammar = child_elements(sense)
        .find(|child| has_class(*child, "subc"))
        .map(text_of)
        .filter(|text| !text.is_empty());
    meaning.notes = child_elements(sense)
        .filter(|child| has_class(*child, "note") || has_class(*child, "re"))
        .map(|note| text_of(note).trim_start_matches('▶').trim().to_owned())
        .filter(|text| !text.is_empty())
        .collect();
    meaning.relations = child_elements(sense).filter(|child| has_class(*child, "thes")).filter_map(synonyms_of).collect();
    Some(meaning)
}

fn explanation_of(sense: ElementRef) -> Option<String> {
    if has_class(sense, "xr") {
        return Some(text_of(sense)).filter(|text| !text.is_empty());
    }
    child_elements(sense)
        .find(|child| has_class(*child, "def"))
        .or_else(|| child_elements(sense).find(|child| has_class(*child, "xr")))
        .map(text_of)
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
            let text = select_first(example, "span.quote").map(text_of).filter(|text| !text.is_empty())?;
            Some(WordUsageExample {
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
            }
        })
        .filter(|term| !term.text.is_empty())
        .collect::<Vec<_>>();
    (!terms.is_empty()).then(|| Relation { kind: "synonym".to_owned(), terms })
}
