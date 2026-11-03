use anyhow::{anyhow, bail};
use scraper::{ElementRef, Html};

use super::html_dom::{select_all, select_first, text_of, text_without};
use super::merriam_webster_fragment::clean_entries;
use super::merriam_webster_sections::sections_of;
use super::merriam_webster_senses::definition_of;
use super::{HtmlParser, WordNotFoundError};
use crate::scraper::{Class, Region, WordClass, WordDefinition, WordForm, WordPronounce};

const NOT_FOUND_TITLE: &str = "Result Not Found";
const UNKNOWN_CLASS: &str = "unknown";
const AUDIO_HOST: &str = "https://media.merriam-webster.com/audio/prons/en";

#[derive(Clone)]
pub struct MerriamWebsterHtmlParser;

impl HtmlParser for MerriamWebsterHtmlParser {
    type Output = WordDefinition;

    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        let document = Html::parse_document(html_content);
        let root = document.root_element();
        let content = select_first(root, "div#left-content").ok_or_else(|| missing_content(root))?;
        let entries = select_all(content, "div.entry-word-section-container");
        let word = entries
            .first()
            .and_then(|entry| select_first(*entry, ".hword"))
            .map(text_of)
            .ok_or_else(|| no_entry(content))?;

        let definition = WordDefinition {
            extracted_html: Some(clean_entries(root)),
            ..WordDefinition::new(
                word,
                entries.iter().map(|entry| class_of(*entry)).collect(),
                sections_of(root),
            )
        };
        if !definition.has_meanings() {
            bail!(WordNotFoundError);
        }
        Ok(definition)
    }
}

fn class_of(entry: ElementRef) -> WordClass {
    let class_name = select_first(entry, "h2.parts-of-speech")
        .map(text_of)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| UNKNOWN_CLASS.to_owned());
    let mut class = WordClass::new(
        Class::from(class_name.to_lowercase().as_str()),
        vec![definition_of(entry)],
        select_all(entry, "a.play-pron-v2").into_iter().filter_map(pronunciation_of).collect(),
    );
    class.forms = select_all(entry, "span.vg-ins span.if")
        .into_iter()
        .map(text_of)
        .filter(|text| !text.is_empty())
        .map(|text| WordForm { label: None, text, audio: None, pronunciations: Vec::new() })
        .collect();
    class
}

fn pronunciation_of(link: ElementRef) -> Option<WordPronounce> {
    let region = match link.value().attr("data-lang")? {
        "en_us" => Region::US,
        "en_uk" => Region::UK,
        _ => return None,
    };
    let folder = if region == Region::US { "us" } else { "uk" };
    let dir = link.value().attr("data-dir")?;
    let file = link.value().attr("data-file")?;
    Some(WordPronounce {
        region,
        ipa: text_without(link, &|part| part.value().name() == "svg"),
        link: format!("{AUDIO_HOST}/{folder}/mp3/{dir}/{file}.mp3"),
    })
}

fn missing_content(root: ElementRef) -> anyhow::Error {
    let title = select_first(root, "title").map(text_of).unwrap_or_default();
    if title.contains(NOT_FOUND_TITLE) || select_first(root, "h1.mispelled-word").is_some() {
        anyhow::Error::new(WordNotFoundError)
    } else {
        layout_changed()
    }
}

fn no_entry(content: ElementRef) -> anyhow::Error {
    if select_first(content, "p.partial-match-blurb").is_some() {
        anyhow::Error::new(WordNotFoundError)
    } else {
        layout_changed()
    }
}

fn layout_changed() -> anyhow::Error {
    anyhow!("Merriam-Webster page has no readable entry; the layout may have changed")
}
