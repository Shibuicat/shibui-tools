use scraper::ElementRef;

use super::html_dom::{has_class, select_all};
use super::wiktionary_fragment::clean_html_with;

const KEPT_ATTRIBUTES: [&str; 9] = ["class", "href", "title", "id", "data-lang", "data-file", "data-dir", "data-hom", "lang"];
const KEPT_BLOCKS: &str = "div.entry-word-section-container, div#word-history, div#synonyms, div#related-phrases, div#related-rhymes";
const NOISE_TAGS: [&str; 4] = ["svg", "button", "img", "mw-icon-new-badge"];

pub fn clean_entries(root: ElementRef) -> String {
    let blocks = select_all(root, KEPT_BLOCKS)
        .into_iter()
        .map(|block| clean_html_with(block, &KEPT_ATTRIBUTES, &is_noise))
        .collect::<String>();
    format!("<div id=\"left-content\">{blocks}</div>")
}

fn is_noise(element: ElementRef) -> bool {
    NOISE_TAGS.contains(&element.value().name())
        || element.value().name().starts_with("mw-")
        || has_class(element, "vocab-builder-cta-btn-container")
        || has_class(element, "vocab-builder-web-components")
}
