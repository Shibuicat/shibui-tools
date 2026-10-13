use scraper::ElementRef;

use super::html_dom::has_class;
use super::wiktionary_fragment::clean_html_with;

const KEPT_ATTRIBUTES: [&str; 14] = [
    "class", "href", "title", "id", "type", "data-src-mp3", "data-src-ogg", "cefr", "unbox", "form",
    "geo", "sensenum", "xt", "src",
];
const NOISE_CLASSES: [&str; 4] = ["am-entry_long", "am-entry", "dictlink-g", "pron-link"];

pub fn clean_entry(content: ElementRef) -> String {
    clean_html_with(content, &KEPT_ATTRIBUTES, &is_noise)
}

fn is_noise(element: ElementRef) -> bool {
    element.value().name() == "img"
        || element.value().id() == Some("ring-links-box")
        || NOISE_CLASSES.iter().any(|class| has_class(element, class))
}
