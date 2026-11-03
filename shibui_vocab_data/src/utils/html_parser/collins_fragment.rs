use scraper::ElementRef;

use super::html_dom::{has_class, select_all};
use super::wiktionary_fragment::clean_html_with;

const KEPT_ATTRIBUTES: [&str; 9] = ["class", "href", "id", "title", "lang", "data-src-mp3", "data-lang", "data-band", "data-type"];
const NOISE_TAGS: [&str; 4] = ["svg", "img", "iframe", "video"];
const NOISE_CLASSES: [&str; 4] = ["socialButtons", "contentslot", "am-dictionary_long", "video"];

pub fn clean_entries(root: ElementRef) -> String {
    let nested = select_all(root, "div.dictionaries > div.dictionary");
    let wrappers = if nested.is_empty() { select_all(root, "div.dictionaries") } else { nested };
    let blocks = wrappers
        .into_iter()
        .map(|block| clean_html_with(block, &KEPT_ATTRIBUTES, &is_noise))
        .collect::<String>();
    format!("<div id=\"collins\">{blocks}</div>")
}

fn is_noise(element: ElementRef) -> bool {
    NOISE_TAGS.contains(&element.value().name()) || NOISE_CLASSES.iter().any(|class| has_class(element, class))
}
