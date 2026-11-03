use scraper::ElementRef;

use super::html_dom::{path_of, select_first, text_of};
use crate::scraper::LinkKind;

const WORD_PATH: &str = "/dictionary/english/";
const NOISE_CLASSES: [&str; 3] = ["xref", "hax", "dxref-w"];

pub fn xref_kind(xref: ElementRef) -> Option<String> {
    xref.value()
        .classes()
        .find(|class| !NOISE_CLASSES.contains(class) && !class.starts_with("lm") && !class.starts_with("lp"))
        .map(normalise_kind)
}

fn normalise_kind(raw: &str) -> String {
    match raw {
        "synonyms" => "synonym".to_owned(),
        "opposite" | "opposites" => "antonym".to_owned(),
        "idioms" => "idiom".to_owned(),
        "phrasal_verbs" | "phrasal_verb" => "phrasal-verb".to_owned(),
        "related_words" | "related_word" => "related".to_owned(),
        other => other.replace('_', "-"),
    }
}

pub fn link_of(href: &str, text: &str) -> (Option<String>, Option<LinkKind>) {
    let href = href.trim();
    match href {
        "" => (None, None),
        _ if href.contains(WORD_PATH) => (Some(text.to_owned()), Some(LinkKind::Word)),
        _ => (Some(path_of(href)), Some(LinkKind::Path)),
    }
}

pub fn anchor_link(scope: ElementRef, text: &str) -> (Option<String>, Option<LinkKind>) {
    select_first(scope, "a")
        .and_then(|anchor| anchor.value().attr("href"))
        .map(|href| link_of(href, text))
        .unwrap_or((None, None))
}

pub fn non_empty_text(element: Option<ElementRef>) -> Option<String> {
    element.map(text_of).filter(|text| !text.is_empty())
}
