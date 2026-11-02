use scraper::ElementRef;

use super::html_dom::{has_class, select_all};
use super::wiktionary_fragment::clean_html_with;

const KEPT_ATTRIBUTES: [&str; 8] = ["class", "href", "id", "title", "lang", "data-audiosrc", "data-audioorigin", "aria-label"];
const KEPT_BLOCKS: &str = "section.sec-item-entry-group, section.sec-list-notes, section.sec-other-word-forms, section.sec-history-origin, section.sec-explanation, section.sec-example-sentences, section.sec-related-words";
const NOISE_TAGS: [&str; 3] = ["svg", "img", "iframe"];

pub fn clean_entries(root: ElementRef) -> String {
    let blocks = select_all(root, KEPT_BLOCKS)
        .into_iter()
        .map(|block| clean_html_with(block, &KEPT_ATTRIBUTES, &is_noise))
        .collect::<String>();
    format!("<div id=\"dictionary-com\">{blocks}</div>")
}

fn is_noise(element: ElementRef) -> bool {
    NOISE_TAGS.contains(&element.value().name())
        || ["box-additional", "box-by-dcom", "box-source-logo"].iter().any(|class| has_class(element, class))
}
