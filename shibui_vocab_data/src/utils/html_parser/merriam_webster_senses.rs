use scraper::ElementRef;

use super::html_dom::{select_all, select_first, text_of};
use crate::scraper::{ClassDefinition, Label, WordContext, WordExplanation, WordUsageExample};

pub fn definition_of(entry: ElementRef) -> ClassDefinition {
    ClassDefinition {
        contexts: select_all(entry, "div.vg")
            .into_iter()
            .flat_map(|group| {
                let heading = select_first(group, "p.vd").map(text_of).filter(|text| !text.is_empty());
                select_all(group, "div.sb-entry")
                    .into_iter()
                    .filter_map(move |sense| context_of(heading.clone(), sense))
                    .collect::<Vec<_>>()
            })
            .collect(),
    }
}

fn context_of(heading: Option<String>, sense: ElementRef) -> Option<WordContext> {
    let explanation = select_all(sense, "span.dtText")
        .into_iter()
        .map(text_of)
        .map(|text| text.trim_start_matches(':').trim().to_owned())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    if explanation.is_empty() {
        return None;
    }

    let mut meaning = WordExplanation::new(explanation, examples_of(sense));
    meaning.labels = select_all(sense, "span.sl")
        .into_iter()
        .map(|label| Label { kind: "status".to_owned(), text: text_of(label) })
        .filter(|label| !label.text.is_empty())
        .collect();
    meaning.notes = select_all(sense, "span.un").into_iter().map(text_of).filter(|note| !note.is_empty()).collect();

    Some(WordContext { description: heading, meanings: vec![meaning] })
}

fn examples_of(sense: ElementRef) -> Vec<WordUsageExample> {
    select_all(sense, "span.ex-sent")
        .into_iter()
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(WordUsageExample::plain)
        .collect()
}
