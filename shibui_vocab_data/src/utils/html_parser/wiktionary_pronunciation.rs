use scraper::{ElementRef, Selector};

use super::wiktionary_dom::{child_elements, text_of};
use crate::scraper::{Region, WordPronounce};

const ORDER: [Region; 2] = [Region::UK, Region::US];

struct PronunciationLine {
    regions: Vec<Region>,
    ipa: Option<String>,
    audio: Option<String>,
}

pub fn pronounces_of(section: ElementRef) -> Vec<WordPronounce> {
    let lines = child_elements(section)
        .filter(|child| child.value().name() == "ul")
        .flat_map(child_elements)
        .map(line_of)
        .collect::<Vec<_>>();

    ORDER
        .iter()
        .filter_map(|region| pronounce_for(region, &lines))
        .collect()
}

fn pronounce_for(region: &Region, lines: &[PronunciationLine]) -> Option<WordPronounce> {
    let matching = lines.iter().filter(|line| line.regions.contains(region));
    let ipa = matching.clone().find_map(|line| line.ipa.clone());
    let link = matching.clone().find_map(|line| line.audio.clone());
    (ipa.is_some() || link.is_some()).then(|| WordPronounce {
        region: region.clone(),
        ipa: ipa.unwrap_or_default(),
        link: link.unwrap_or_default(),
    })
}

fn line_of(item: ElementRef) -> PronunciationLine {
    PronunciationLine {
        regions: regions_of(accent_label(item).as_deref()),
        ipa: first_text(item, "span.IPA"),
        audio: audio_path(item),
    }
}

fn accent_label(item: ElementRef) -> Option<String> {
    first_text(item, "span.usage-label-accent")
}

fn first_text(item: ElementRef, css: &str) -> Option<String> {
    let selector = Selector::parse(css).unwrap();
    item.select(&selector).next().map(text_of).filter(|text| !text.is_empty())
}

fn regions_of(label: Option<&str>) -> Vec<Region> {
    match label {
        None => ORDER.to_vec(),
        Some(label) => {
            let lowered = label.to_lowercase();
            let words = lowered.split(|c: char| !c.is_alphanumeric()).collect::<Vec<_>>();
            let uk = words.contains(&"uk") || words.contains(&"british") || words.contains(&"received") || words.contains(&"rp");
            let us = words.contains(&"us") || words.contains(&"american") || words.contains(&"ga");
            ORDER
                .iter()
                .filter(|region| match region {
                    Region::UK => uk,
                    Region::US => us,
                })
                .cloned()
                .collect()
        }
    }
}

fn audio_path(item: ElementRef) -> Option<String> {
    let selector = Selector::parse("audio source[type=\"audio/mpeg\"]").unwrap();
    item.select(&selector)
        .next()
        .and_then(|source| source.value().attr("src"))
        .map(path_of)
}

fn path_of(source: &str) -> String {
    let without_query = source.split('?').next().unwrap_or(source);
    let without_scheme = without_query
        .strip_prefix("https:")
        .or_else(|| without_query.strip_prefix("http:"))
        .unwrap_or(without_query);
    match without_scheme.strip_prefix("//") {
        Some(rest) => rest.find('/').map(|slash| rest[slash..].to_owned()).unwrap_or_default(),
        None => without_scheme.to_owned(),
    }
}
