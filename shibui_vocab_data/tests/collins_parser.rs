use shibui_vocab_data::scraper::WordDefinition;
use shibui_vocab_data::utils::html_parser::{collins_parser::CollinsHtmlParser, HtmlParser, WordNotFoundError};

fn parse(html: &str) -> anyhow::Result<WordDefinition> {
    CollinsHtmlParser.parse(html)
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
fn the_first_dictionary_gives_the_classes_and_the_others_become_groups() {
    let definition = parse_fixture("collins_abandon");

    assert_eq!(definition["word"], "abandon");
    assert_eq!(definition["classes"][0]["className"], "Verb");
    let keys: Vec<_> = definition["groups"].as_array().unwrap().iter().map(|g| g["key"].as_str().unwrap()).collect();
    assert!(keys.contains(&"Collins_Eng_Dict"));
    let english = definition["groups"].as_array().unwrap().iter().find(|g| g["key"] == "Collins_Eng_Dict").unwrap();
    assert_eq!(english["kind"], "dictionary");
    assert_eq!(english["label"], "Collins English Dictionary");
    assert!(english["attribution"].as_str().unwrap().contains("HarperCollins"));
}

#[test]
fn several_entries_of_one_dictionary_get_numbered_group_keys() {
    let definition = parse_fixture("collins_back");

    let keys: Vec<_> = definition["groups"].as_array().unwrap().iter().map(|g| g["key"].as_str().unwrap()).collect();
    let unique: std::collections::HashSet<_> = keys.iter().collect();
    assert_eq!(keys.len(), unique.len());
    assert!(keys.contains(&"Cob_Adv_Brit-2"));
}

#[test]
fn senses_of_the_same_class_are_merged_with_their_own_level_and_pattern() {
    let definition = parse_fixture("collins_back");

    let first = &definition["classes"][0];
    assert!(meanings(first).len() > 3);
    assert_eq!(meanings(first)[0]["level"], "B1");
    assert!(meanings(first)[0]["pattern"].as_str().unwrap().contains("ADVERB"));
}

#[test]
fn pronunciations_and_forms_carry_audio_links() {
    let definition = parse_fixture("collins_abandon");

    let pronunciation = &definition["classes"][0]["pronunciations"][0];
    assert_eq!(pronunciation["region"], "UK");
    assert!(pronunciation["link"].as_str().unwrap().ends_with(".mp3"));
    let forms = definition["classes"][0]["forms"].as_array().unwrap();
    assert_eq!(forms[0]["text"], "abandons");
    assert_eq!(forms[2]["label"], "past tense, past participle");
    assert!(forms[2]["audio"].as_str().unwrap().ends_with(".mp3"));
}

#[test]
fn examples_keep_their_grammar_pattern() {
    let definition = parse_fixture("collins_abandon");

    let example = &meanings(&definition["classes"][0])[0]["examples"][0];
    assert_eq!(example["text"], "He claimed that his parents had abandoned him.");
    assert_eq!(example["pattern"], "[VERB noun]");
}

#[test]
fn synonyms_become_relations_with_links() {
    let definition = parse_fixture("collins_abandon");

    let relation = &meanings(&definition["classes"][0])[0]["relations"][0];
    assert_eq!(relation["kind"], "synonym");
    assert_eq!(relation["terms"][0]["text"], "leave");
    assert!(relation["terms"][0]["link"].as_str().unwrap().contains("/dictionary/english/leave"));
}

#[test]
fn word_origin_is_a_section_of_its_group() {
    let definition = parse_fixture("collins_abandon");

    let english = definition["groups"].as_array().unwrap().iter().find(|g| g["key"] == "Collins_Eng_Dict").unwrap();
    let etymology = english["sections"].as_array().unwrap().iter().find(|s| s["key"] == "etymology").unwrap();
    assert!(etymology["items"][0]["text"].as_str().unwrap().contains("Old French"));
}

#[test]
fn a_page_with_the_no_results_text_is_not_found() {
    let error = parse("<html><body><p>Sorry, no results for \"zzzz\" in the English Dictionary.</p></body></html>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn a_page_without_entries_and_without_the_text_is_a_layout_change() {
    let error = parse("<html><body><p>Hello</p></body></html>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_none());
}
