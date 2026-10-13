use scraper::ElementRef;

use super::html_dom::{child_elements, has_class, path_of, select_first, text_of};
use crate::scraper::{Region, WordPronounce};

pub fn pronounces_of(webtop: ElementRef) -> Vec<WordPronounce> {
    child_elements(webtop)
        .filter(|child| has_class(*child, "phonetics"))
        .flat_map(child_elements)
        .filter_map(pronounce_of)
        .collect()
}

fn pronounce_of(accent: ElementRef) -> Option<WordPronounce> {
    let region = if has_class(accent, "phons_br") {
        Region::UK
    } else if has_class(accent, "phons_n_am") {
        Region::US
    } else {
        return None;
    };
    let ipa = child_elements(accent).find(|child| has_class(*child, "phon")).map(text_of)?;
    let link = select_first(accent, "div.sound")
        .and_then(|sound| sound.value().attr("data-src-mp3"))
        .map(path_of)
        .unwrap_or_default();
    Some(WordPronounce { region, ipa, link })
}
