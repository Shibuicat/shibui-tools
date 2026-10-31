use shibui_vocab_data::scraper::WordDefinition;
use shibui_vocab_data::utils::html_parser::{
    cambridge_parser::CambridgeHtmlParser, HtmlParser, WordNotFoundError,
};

fn parse(html: &str) -> anyhow::Result<WordDefinition> {
    CambridgeHtmlParser.parse(html)
}

fn parse_fixture(name: &str) -> WordDefinition {
    let html = std::fs::read_to_string(format!("tests/fixtures/{name}.html")).unwrap();
    parse(&html).unwrap()
}

fn as_json(definition: &WordDefinition) -> serde_json::Value {
    serde_json::to_value(definition).unwrap()
}

#[test]
fn entry_with_pr_class_keeps_parsing_as_before() {
    let definition = as_json(&parse_fixture("activator"));

    assert_eq!(definition["word"], "activator");
    assert_eq!(definition["classes"][0]["className"], "Noun");
}

#[test]
fn entry_without_pr_class_is_parsed() {
    let definition = as_json(&parse_fixture("accede"));

    assert_eq!(definition["word"], "accede");
    assert!(!definition["classes"][0]["definitions"][0]["contexts"][0]["meanings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn phrase_style_page_is_parsed_as_a_phrase_class() {
    let definition = as_json(&parse_fixture("ablutions"));

    assert_eq!(definition["word"], "ablutions");
    assert_eq!(definition["classes"][0]["className"], "phrase");
    assert_eq!(definition["classes"][0]["pronounces"], serde_json::json!([]));
    assert!(definition["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0]
        ["explanation"]
        .as_str()
        .unwrap()
        .starts_with("Your ablutions are"));
}

#[test]
fn extracted_html_parses_back_to_the_same_definition() {
    ["activator", "accede", "ablutions"].iter().for_each(|name| {
        let first = parse_fixture(name);
        let second = parse(first.extracted_html.as_ref().unwrap()).unwrap();

        assert_eq!(as_json(&first), as_json(&second), "{name}");
    });
}

#[test]
fn page_without_any_entry_is_word_not_found() {
    let error = parse("<html><body><h1>Nothing here</h1></body></html>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn entry_without_any_meaning_is_word_not_found() {
    let html = r#"<div class="pr entry-body__el"><div class="pos-header dpos-h"><span class="hw dhw">x</span></div></div>"#;

    let error = parse(html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

const ACTIVATOR_PRONUNCIATIONS_START: &str = r#"<div></div><span class="uk dpron-i ">"#;
const ACTIVATOR_PRONUNCIATIONS_END: &str = r#"</div><div class="pos-body">"#;

fn block(region: &str, ipa: &str, audio: &str) -> String {
    let sound = if audio.is_empty() {
        String::new()
    } else {
        format!(r#"<audio class="hdn"><source type="audio/mpeg" src="{audio}"><source type="audio/ogg" src="{audio}.ogg"></audio>"#)
    };
    let ipa = if ipa.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="pron dpron">/<span class="ipa dipa lpr-2 lpl-1">{ipa}</span>/</span>"#)
    };
    format!(r#"<span class="{region} dpron-i "><span class="region dreg">{region}</span>{sound}{ipa}</span>"#)
}

fn activator_with_pronunciations(blocks: &[String]) -> WordDefinition {
    let html = std::fs::read_to_string("tests/fixtures/activator.html").unwrap();
    let start = html.find(ACTIVATOR_PRONUNCIATIONS_START).unwrap();
    let end = html.find(ACTIVATOR_PRONUNCIATIONS_END).unwrap();
    let page = format!("{}<div></div>{}{}", &html[..start], blocks.join(" "), &html[end..]);
    parse(&page).unwrap()
}

fn pronounces(definition: &WordDefinition) -> serde_json::Value {
    as_json(definition)["classes"][0]["pronounces"].clone()
}

#[test]
fn each_region_takes_the_audio_of_its_own_block() {
    let definition = activator_with_pronunciations(&[
        block("uk", "uk-ipa", "/media/english/uk_pron/a.mp3"),
        block("us", "us-ipa", "/media/english/us_pron/a.mp3"),
    ]);

    assert_eq!(
        pronounces(&definition),
        serde_json::json!([
            {"region": "UK", "ipa": "uk-ipa", "link": "/media/english/uk_pron/a.mp3"},
            {"region": "US", "ipa": "us-ipa", "link": "/media/english/us_pron/a.mp3"},
        ])
    );
}

#[test]
fn uk_block_without_audio_does_not_borrow_the_us_audio() {
    let definition = activator_with_pronunciations(&[
        block("uk", "uk-ipa", ""),
        block("us", "us-ipa", "/media/english/us_pron/a.mp3"),
    ]);

    assert_eq!(
        pronounces(&definition),
        serde_json::json!([
            {"region": "UK", "ipa": "uk-ipa", "link": ""},
            {"region": "US", "ipa": "us-ipa", "link": "/media/english/us_pron/a.mp3"},
        ])
    );
}

#[test]
fn uk_block_with_audio_but_no_ipa_is_kept() {
    let definition = activator_with_pronunciations(&[
        block("uk", "", "/media/english/uk_pron/a.mp3"),
        block("us", "us-ipa", "/media/english/us_pron/a.mp3"),
    ]);

    assert_eq!(pronounces(&definition)[0]["link"], "/media/english/uk_pron/a.mp3");
    assert_eq!(pronounces(&definition)[0]["region"], "UK");
}

#[test]
fn every_block_of_a_region_is_kept_and_identical_ones_are_not_repeated() {
    let definition = activator_with_pronunciations(&[
        block("uk", "first", "/media/english/uk_pron/a.mp3"),
        block("uk", "second", "/media/english/uk_pron/b.mp3"),
        block("uk", "second", "/media/english/uk_pron/b.mp3"),
        block("us", "us-ipa", "/media/english/us_pron/a.mp3"),
    ]);

    let regions_and_ipas: Vec<String> = pronounces(&definition)
        .as_array()
        .unwrap()
        .iter()
        .map(|p| format!("{} {}", p["region"].as_str().unwrap(), p["ipa"].as_str().unwrap()))
        .collect();
    assert_eq!(regions_and_ipas, vec!["UK first", "UK second", "US us-ipa"]);
}

#[test]
fn a_region_with_no_block_adds_no_empty_entry() {
    let definition = activator_with_pronunciations(&[block("uk", "uk-ipa", "/media/english/uk_pron/a.mp3")]);

    assert_eq!(pronounces(&definition).as_array().unwrap().len(), 1);
}
