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

#[test]
fn a_redirect_to_a_compound_phrase_page_without_an_entry_headword_is_not_found() {
    let html = r#"<html><head><title>ACADEMIC RESEARCHER definition and meaning | Collins English Dictionary</title></head><body><div class="page"><div class="dictionary Collocations_EN dictentry"><h1 class="entry_title">Example sentences academic researcher</h1></div><div class="dictionary Cob_Adv_Brit dictentry"><h2 class="entry_title"><span class="orth">academic</span></h2></div></div></body></html>"#;

    let error = CollinsHtmlParser.parse(html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn a_spelling_variant_that_only_points_to_another_word_is_not_found() {
    let html = r#"<html><head><title>ORGANISATION definition and meaning | Collins English Dictionary</title></head><body><div class="dictentry dictlink"><div class="cB-h"><h2 class="h2_entry"><span class="orth">organisation</span></h2></div><div class="content definitions cobuild br"><span class="xr"><a class="ref" href="/dictionary/english/organization">organization</a></span></div></div></body></html>"#;

    let error = CollinsHtmlParser.parse(html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn a_page_whose_entries_sit_in_one_element_that_is_both_dictionaries_and_dictionary_keeps_them_in_the_fragment() {
    let html = std::fs::read_to_string("tests/fixtures/collins_recommending.html").unwrap();

    let first = CollinsHtmlParser.parse(&html).unwrap();
    let fragment = first.extracted_html.clone().unwrap();
    let again = CollinsHtmlParser.parse(&fragment).unwrap();

    assert!(fragment.contains("dictentry"));
    assert!(fragment.len() > 1000);
    assert_eq!(serde_json::to_value(&first).unwrap(), serde_json::to_value(&again).unwrap());
}

#[test]
fn an_entry_whose_only_senses_sit_under_a_phrase_form_uses_them_as_its_meanings() {
    let html = r#"<html><head><title>COLONIES definition and meaning | Collins English Dictionary</title></head><body><div class="dictionaries dictionary Collins_Eng_Dict"><div class="dictentry dictlink"><h2 class="h2_entry"><span class="orth">Colonies</span></h2><div class="content definitions"><div class="hom"><span class="gramGrp pos">plural noun</span><div class="re type-idm"><span class="form type-idm orth">the Colonies</span><div class="sense"><span class="sensenum">1. </span><span class="lbl type-geo"> British</span><div class="def">the subject territories formerly in the British Empire</div></div><div class="sense"><span class="sensenum">2. </span><span class="lbl type-geo"> US</span><div class="def">the 13 states forming the original United States</div></div></div></div><div class="copyright">Collins</div></div></div></div></body></html>"#;

    let definition = serde_json::to_value(CollinsHtmlParser.parse(html).unwrap()).unwrap();

    let contexts = definition["classes"][0]["definitions"][0]["contexts"].as_array().unwrap();
    assert_eq!(contexts[0]["description"], "the Colonies");
    assert_eq!(contexts[0]["meanings"].as_array().unwrap().len(), 2);
    assert_eq!(contexts[0]["meanings"][1]["explanation"], "the 13 states forming the original United States");
}

#[test]
fn the_headword_comes_from_the_first_block_that_has_one_even_when_an_examples_block_comes_first() {
    let html = r#"<html><head><title>MONKEYPOX definition and meaning | Collins English Dictionary</title></head><body><div class="page"><div class="dictionary Scientific dictentry"><h1 class="entry_title">Example sentences monkeypox</h1></div><div class="dictionary Collins_Eng_Dict dictentry"><h2 class="h2_entry"><span class="orth">monkeypox</span></h2><div class="content definitions"><div class="hom"><span class="gramGrp pos">uncountable noun</span><div class="sense"><div class="def">a viral disease</div></div></div></div></div></div></body></html>"#;

    let definition = CollinsHtmlParser.parse(html).unwrap();

    assert_eq!(definition.word, "monkeypox");
}
