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
