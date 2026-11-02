use shibui_vocab_data::scraper::WordDefinition;
use shibui_vocab_data::utils::html_parser::{
    dictionary_com_parser::DictionaryComHtmlParser, HtmlParser, WordNotFoundError,
};

fn parse(html: &str) -> anyhow::Result<WordDefinition> {
    DictionaryComHtmlParser.parse(html)
}

fn parse_fixture(name: &str) -> serde_json::Value {
    let html = std::fs::read_to_string(format!("tests/fixtures/{name}.html")).unwrap();
    serde_json::to_value(parse(&html).unwrap()).unwrap()
}

fn meanings(class: &serde_json::Value) -> Vec<&serde_json::Value> {
    class["definitions"][0]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|context| context["meanings"].as_array().unwrap().iter())
        .collect()
}

#[test]
fn the_american_block_gives_the_classes_and_the_british_block_becomes_a_group() {
    let definition = parse_fixture("dcom_abandon");

    assert_eq!(definition["word"], "abandon");
    assert_eq!(definition["classes"][0]["className"], "Verb");
    assert_eq!(definition["classes"][0]["subClass"], "used with object");
    let british = definition["groups"].as_array().unwrap().iter().find(|group| group["key"] == "british").unwrap();
    assert_eq!(british["kind"], "region");
    assert!(!british["classes"].as_array().unwrap().is_empty());
}

#[test]
fn pronunciation_links_point_at_the_audio_host_and_carry_the_region() {
    let definition = parse_fixture("dcom_abandon");

    let american = &definition["classes"][0]["pronunciations"][0];
    assert_eq!(american["region"], "US");
    assert!(american["link"].as_str().unwrap().starts_with("https://audio.dictionary.com/tts/"));
    assert!(!american["ipa"].as_str().unwrap().contains('/'));
    let british = definition["groups"].as_array().unwrap().iter().find(|group| group["key"] == "british").unwrap();
    assert_eq!(british["classes"][0]["pronunciations"][0]["region"], "UK");
}

#[test]
fn inflected_forms_keep_their_labels() {
    let definition = parse_fixture("dcom_abandon");

    let forms = definition["classes"][0]["forms"].as_array().unwrap();
    assert_eq!(forms[0]["text"], "abandons");
    assert_eq!(forms[0]["label"], "present (3rd person singular)");
    assert_eq!(forms[1]["text"], "abandoned");
}

#[test]
fn field_labels_synonyms_and_antonyms_are_typed() {
    let definition = parse_fixture("dcom_abandon");

    let all = meanings(&definition["classes"][0]);
    assert!(all.iter().any(|meaning| meaning["labels"][0]["text"] == "Law"));
    let with_synonyms = all.iter().find(|meaning| meaning["relations"][0]["kind"] == "synonym").unwrap();
    assert_eq!(with_synonyms["relations"][0]["terms"][0]["text"], "abdicate");
    assert!(all.iter().all(|meaning| !meaning["explanation"].as_str().unwrap().is_empty()));
}

#[test]
fn nested_definitions_become_sub_meanings() {
    let definition = parse_fixture("dcom_back");

    let nested = definition["classes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|class| meanings(class))
        .find(|meaning| meaning["explanation"] == "Carpentry")
        .unwrap();
    assert!(nested["subMeanings"].as_array().unwrap().len() >= 2);
}

#[test]
fn the_idioms_block_becomes_a_group_section_of_links() {
    let definition = parse_fixture("dcom_back");

    let idioms = definition["groups"].as_array().unwrap().iter().find(|group| group["key"] == "idiom").unwrap();
    assert!(idioms["classes"].as_array().unwrap().is_empty());
    assert_eq!(idioms["sections"][0]["items"][0]["linkKind"], "word");
}

#[test]
fn etymology_forms_related_words_and_examples_are_sections() {
    let definition = parse_fixture("dcom_abandon");

    let keys: Vec<_> = definition["sections"].as_array().unwrap().iter().map(|s| s["key"].as_str().unwrap()).collect();
    for expected in ["etymology", "word-forms", "related", "examples"] {
        assert!(keys.contains(&expected), "{expected} missing in {keys:?}");
    }
}

#[test]
fn a_page_without_entries_and_a_not_found_title_is_not_found() {
    let error = parse("<html><head><title>Page Not Found | Dictionary.com</title></head><body></body></html>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn a_page_without_entries_and_another_title_is_a_layout_change() {
    let error = parse("<html><head><title>Dictionary.com</title></head><body></body></html>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_none());
}
