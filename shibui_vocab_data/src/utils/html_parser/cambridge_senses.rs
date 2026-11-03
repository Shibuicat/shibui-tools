use scraper::ElementRef;

use super::cambridge_terms::{anchor_link, non_empty_text, xref_kind};
use super::html_dom::{has_ancestor_class, has_class, select_all, select_first, text_of, text_without};
use crate::scraper::{
    ClassDefinition, Label, Relation, RelationTerm, WordContext, WordExplanation, WordUsageExample,
};

const VARIANT_CLASSES: [&str; 3] = ["spellvar", "irreg-infls", "var"];

pub fn has_own_meanings(entry: ElementRef) -> bool {
    !meanings_of(entry).is_empty()
}

pub fn definitions_of(entry: ElementRef) -> Vec<ClassDefinition> {
    if has_own_meanings(entry) {
        sense_definitions_of(entry)
    } else {
        vec![phrase_definition_of(entry)]
    }
}

fn phrase_definition_of(entry: ElementRef) -> ClassDefinition {
    ClassDefinition {
        contexts: select_all(entry, "div.phrase-block")
            .into_iter()
            .map(|block| WordContext {
                description: non_empty_text(select_first(block, "span.phrase-title")),
                meanings: select_all(block, "div.def-block").into_iter().filter_map(meaning_of).collect(),
            })
            .filter(|context| !context.meanings.is_empty())
            .collect(),
    }
}

fn sense_definitions_of(entry: ElementRef) -> Vec<ClassDefinition> {
    let bodies = select_all(entry, "div.pos-body");
    if bodies.is_empty() {
        vec![definition_of(entry)]
    } else {
        bodies.into_iter().map(definition_of).collect()
    }
}

fn definition_of(scope: ElementRef) -> ClassDefinition {
    ClassDefinition {
        contexts: contexts_of(scope)
            .into_iter()
            .filter(|context| !context.meanings.is_empty())
            .collect(),
    }
}

fn contexts_of(scope: ElementRef) -> Vec<WordContext> {
    let senses = select_all(scope, "div.dsense");
    if senses.is_empty() {
        return vec![WordContext { description: None, meanings: meanings_of(scope) }];
    }
    senses.into_iter().map(context_of).collect()
}

fn context_of(sense: ElementRef) -> WordContext {
    let heading_grammar = select_first(sense, "h3.dsense_h span.gram").map(text_of);
    WordContext {
        description: non_empty_text(select_first(sense, "span.guideword"))
            .map(|text| text.trim_matches(|c| c == '(' || c == ')').trim().to_owned())
            .filter(|text| !text.is_empty()),
        meanings: meanings_of(sense)
            .into_iter()
            .map(|meaning| WordExplanation {
                grammar: meaning.grammar.or_else(|| heading_grammar.clone()),
                ..meaning
            })
            .collect(),
    }
}

fn meanings_of(scope: ElementRef) -> Vec<WordExplanation> {
    select_all(scope, "div.def-block")
        .into_iter()
        .filter(|block| !has_ancestor_class(*block, "phrase-block") && !has_ancestor_class(*block, "runon"))
        .filter_map(meaning_of)
        .collect()
}

pub fn meaning_of(block: ElementRef) -> Option<WordExplanation> {
    let definition = select_first(block, "div.def")?;
    let explanation = text_without(definition, &is_label_only)
        .trim_end_matches(':')
        .trim()
        .to_owned();
    if explanation.is_empty() {
        return None;
    }
    let info = select_first(block, "span.def-info");
    Some(WordExplanation {
        examples: examples_of(block),
        labels: info
            .map(labels_of)
            .unwrap_or_default()
            .into_iter()
            .chain(definition_labels_of(definition))
            .collect(),
        grammar: non_empty_text(info.and_then(|info| select_first(info, "span.gram"))),
        level: non_empty_text(info.and_then(|info| select_first(info, "span.epp-xref"))),
        notes: info.map(notes_of).unwrap_or_default(),
        relations: relations_of(block),
        ..WordExplanation::new(explanation, Vec::new())
    })
}

fn is_label_only(element: ElementRef) -> bool {
    let usages = select_all(element, "span.usage").into_iter().map(text_of).collect::<Vec<_>>();
    has_class(element, "lab") && !usages.is_empty() && text_of(element) == usages.join(" ") && is_last_in_definition(element)
}

fn is_last_in_definition(element: ElementRef) -> bool {
    let closes_definition = element.parent().and_then(ElementRef::wrap).is_some_and(|parent| has_class(parent, "def"));
    closes_definition
        && element.next_siblings().all(|node| node.value().as_text().is_some_and(|text| text.trim().is_empty()))
}

fn definition_labels_of(definition: ElementRef) -> Vec<Label> {
    select_all(definition, "span.lab")
        .into_iter()
        .filter(|lab| is_label_only(*lab))
        .flat_map(|lab| select_all(lab, "span.usage"))
        .map(|usage| Label { kind: "usage".to_owned(), text: text_of(usage) })
        .collect()
}

fn inside_variant(element: ElementRef) -> bool {
    VARIANT_CLASSES.iter().any(|class| has_class(element, class) || has_ancestor_class(element, class))
}

fn labels_of(info: ElementRef) -> Vec<Label> {
    info.descendants()
        .filter_map(ElementRef::wrap)
        .filter(|element| !inside_variant(*element))
        .filter_map(|element| {
            let kind = ["domain", "usage", "region"].into_iter().find(|kind| has_class(element, kind))?;
            Some(Label { kind: kind.to_owned(), text: text_of(element) })
        })
        .filter(|label| !label.text.is_empty())
        .collect()
}

fn notes_of(info: ElementRef) -> Vec<String> {
    select_all(info, "span.spellvar, span.irreg-infls, span.var")
        .into_iter()
        .map(text_of)
        .filter(|text| !text.is_empty())
        .collect()
}

pub fn examples_of(block: ElementRef) -> Vec<WordUsageExample> {
    select_all(block, "div.examp")
        .into_iter()
        .chain(more_examples(block))
        .filter_map(example_of)
        .collect()
}

fn more_examples(block: ElementRef) -> Vec<ElementRef> {
    let following = block
        .next_siblings()
        .filter_map(ElementRef::wrap)
        .take_while(|sibling| !has_class(*sibling, "def-block"))
        .filter(|sibling| has_class(*sibling, "daccord") && !has_class(*sibling, "smartt"))
        .collect::<Vec<_>>();
    select_all(block, "div.daccord li.eg")
        .into_iter()
        .chain(following.into_iter().flat_map(|accordion| select_all(accordion, "li.eg")))
        .collect()
}

pub fn example_of(container: ElementRef) -> Option<WordUsageExample> {
    let sentence = if has_class(container, "eg") { container } else { select_first(container, "span.eg")? };
    let text = text_without(sentence, &|part| has_class(part, "gloss"));
    (!text.is_empty()).then(|| WordUsageExample {
        pattern: non_empty_text(select_first(container, ".lu"))
            .or_else(|| non_empty_text(select_first(container, "span.gram"))),
        gloss: non_empty_text(select_first(sentence, "span.gloss")),
        ..WordUsageExample::plain(text)
    })
}

fn relations_of(block: ElementRef) -> Vec<Relation> {
    select_all(block, "div.xref")
        .into_iter()
        .filter_map(|xref| {
            let terms = select_all(xref, "div.item").into_iter().filter_map(term_of).collect::<Vec<_>>();
            (!terms.is_empty()).then(|| xref_kind(xref).map(|kind| Relation { kind, terms })).flatten()
        })
        .chain(thesaurus_relation(block))
        .collect()
}

fn thesaurus_relation(block: ElementRef) -> Option<Relation> {
    let terms = select_all(block, "div.daccord li.t-i")
        .into_iter()
        .filter_map(thesaurus_term)
        .collect::<Vec<_>>();
    (!terms.is_empty()).then(|| Relation { kind: "thesaurus".to_owned(), terms })
}

fn thesaurus_term(item: ElementRef) -> Option<RelationTerm> {
    let text = non_empty_text(select_first(item, "a"))?;
    Some(RelationTerm {
        text,
        link: None,
        relevance: None,
        examples: select_all(item, ".dexample")
            .into_iter()
            .map(|example| text_without(example, &|part| has_class(part, "lab")))
            .filter(|example| !example.is_empty())
            .map(WordUsageExample::plain)
            .collect(),
    })
}

fn term_of(item: ElementRef) -> Option<RelationTerm> {
    let text = non_empty_text(select_first(item, "span.x-h"))?;
    Some(RelationTerm { link: anchor_link(item, &text).0, text, relevance: None, examples: Vec::new() })
}
