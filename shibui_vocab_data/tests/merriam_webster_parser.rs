use shibui_vocab_data::scraper::WordDefinition;
use shibui_vocab_data::utils::html_parser::{
    merriam_webster_parser::MerriamWebsterHtmlParser, HtmlParser, WordNotFoundError,
};

fn parse(html: &str) -> anyhow::Result<WordDefinition> {
    MerriamWebsterHtmlParser.parse(html)
}

fn parse_fixture(name: &str) -> WordDefinition {
    parse(&std::fs::read_to_string(format!("tests/fixtures/{name}.html")).unwrap()).unwrap()
}

fn as_json(definition: &WordDefinition) -> serde_json::Value {
    serde_json::to_value(definition).unwrap()
}

#[test]
fn a_page_gives_the_word_and_one_class_per_entry() {
    let definition = as_json(&parse_fixture("mw_resilient"));

    assert_eq!(definition["word"], "resilient");
    assert!(!definition["classes"].as_array().unwrap().is_empty());
    assert_eq!(definition["classes"][0]["className"], "Adjective");
}

#[test]
fn every_class_has_meanings_with_text() {
    let definition = as_json(&parse_fixture("mw_quickly"));

    let meanings: Vec<_> = definition["classes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|class| class["definitions"][0]["contexts"].as_array().unwrap().iter())
        .flat_map(|context| context["meanings"].as_array().unwrap().iter())
        .collect();
    assert!(!meanings.is_empty());
    assert!(meanings.iter().all(|meaning| !meaning["explanation"].as_str().unwrap().is_empty()));
    assert!(meanings.iter().all(|meaning| !meaning["explanation"].as_str().unwrap().starts_with(':')));
}

#[test]
fn pronunciation_audio_is_the_full_address_of_the_mp3_on_the_audio_host() {
    let definition = as_json(&parse_fixture("mw_resilient"));

    let link = definition["classes"][0]["pronunciations"][0]["link"].as_str().unwrap();
    assert!(link.starts_with("https://media.merriam-webster.com/audio/prons/en/us/mp3/"));
    assert!(link.ends_with(".mp3"));
    assert_eq!(definition["classes"][0]["pronunciations"][0]["region"], "US");
    assert!(!definition["classes"][0]["pronunciations"][0]["ipa"].as_str().unwrap().is_empty());
}

#[test]
fn etymology_and_synonyms_become_sections() {
    let definition = as_json(&parse_fixture("mw_resilient"));

    let keys: Vec<_> = definition["sections"].as_array().unwrap().iter().map(|s| s["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"etymology"));
}

#[test]
fn extracted_html_parses_back_to_the_same_definition() {
    ["mw_quickly", "mw_resilient"].iter().for_each(|name| {
        let first = parse_fixture(name);
        let second = parse(first.extracted_html.as_ref().unwrap()).unwrap();

        assert_eq!(as_json(&first), as_json(&second), "{name}");
    });
}

#[test]
fn page_titled_result_not_found_is_word_not_found() {
    let error = parse("<html><head><title>Result Not Found</title></head><body></body></html>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn page_without_the_entry_container_and_without_the_not_found_title_is_a_layout_failure() {
    let error = parse("<html><head><title>Merriam-Webster</title></head><body><p>hello</p></body></html>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_none());
}

#[test]
fn the_page_for_an_unknown_word_is_not_found_even_with_an_ordinary_title() {
    let html = r#"<html><head><title>Hyperbarbarousness - Definition and More from the Free Merriam-Webster Dictionary</title></head><body><h1 class="mispelled-word">&ldquo;hyperbarbarousness&rdquo;</h1><p>The word you&rsquo;ve entered isn&rsquo;t in the dictionary.</p></body></html>"#;

    let error = MerriamWebsterHtmlParser.parse(html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn an_inflected_form_that_only_points_to_its_base_word_is_not_found() {
    let html = r#"<html><head><title>WAS Definition &amp; Meaning - Merriam-Webster</title></head><body><div id="left-content"><div class="entry-word-section-container"><h1 class="hword">was</h1><p class="cxl-ref">past tense first- and third-person singular of be</p></div></div></body></html>"#;

    let error = MerriamWebsterHtmlParser.parse(html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}
