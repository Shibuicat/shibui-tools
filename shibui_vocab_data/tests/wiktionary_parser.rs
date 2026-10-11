use shibui_vocab_data::scraper::WordDefinition;
use shibui_vocab_data::utils::html_parser::{
    wiktionary_parser::WiktionaryHtmlParser, HtmlParser, WordNotFoundError,
};

fn parse(html: &str) -> anyhow::Result<WordDefinition> {
    WiktionaryHtmlParser.parse(html)
}

fn parse_fixture(name: &str) -> serde_json::Value {
    let html = std::fs::read_to_string(format!("tests/fixtures/wiktionary_{name}.html")).unwrap();
    serde_json::to_value(parse(&html).unwrap()).unwrap()
}

fn class_names(definition: &serde_json::Value) -> Vec<String> {
    definition["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|class| class["className"].as_str().unwrap().to_owned())
        .collect()
}

fn section<'a>(definition: &'a serde_json::Value, key: &str, class: Option<&str>) -> &'a serde_json::Value {
    definition["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| {
            section["key"] == key && section.get("className").and_then(|c| c.as_str()) == class
        })
        .unwrap_or_else(|| panic!("no section {key} for {class:?}"))
}

fn item_texts(section: &serde_json::Value) -> Vec<String> {
    section["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["text"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn headword_is_read_from_the_english_section() {
    assert_eq!(parse_fixture("abandon")["word"], "abandon");
}

#[test]
fn every_part_of_speech_of_every_etymology_becomes_a_class_in_page_order() {
    assert_eq!(class_names(&parse_fixture("abandon")), vec!["Verb", "Noun"]);
    assert_eq!(class_names(&parse_fixture("beautiful")), vec!["Adjective", "Noun"]);
}

#[test]
fn a_page_without_etymology_headings_still_gives_its_classes() {
    assert_eq!(class_names(&parse_fixture("although")), vec!["conjunction"]);
}

#[test]
fn plain_senses_share_one_context_without_a_description() {
    let definition = parse_fixture("abandon");
    let context = &definition["classes"][0]["definitions"][0]["contexts"][0];

    assert!(context["description"].is_null());
    let meanings = context["meanings"].as_array().unwrap();
    assert!(meanings.len() >= 3);
    assert!(meanings[0]["explanation"]
        .as_str()
        .unwrap()
        .starts_with("(transitive) To give up or relinquish control of"));
}

#[test]
fn definition_text_leaves_out_citations_references_and_dates() {
    let definition = parse_fixture("abandon");
    let explanation = definition["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0]["explanation"]
        .as_str()
        .unwrap();

    assert!(!explanation.contains("First attested"));
    assert!(!explanation.contains("Macaulay"));
    assert!(!explanation.contains("[1]"));
}

#[test]
fn usage_examples_are_collected_and_quotations_are_not() {
    let definition = parse_fixture("abandon");
    let meanings = definition["classes"][0]["definitions"][0]["contexts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|context| context["meanings"].as_array().unwrap().iter())
        .collect::<Vec<_>>();
    let examples = meanings
        .iter()
        .flat_map(|meaning| meaning["examples"].as_array().unwrap().iter())
        .map(|example| example.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    assert!(examples.iter().all(|example| !example.contains("Macaulay")));
    assert!(!meanings.is_empty());
}

#[test]
fn a_sense_with_sub_senses_becomes_a_context_described_by_the_parent() {
    let definition = parse_fixture("fish");
    let contexts = definition["classes"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|class| class["definitions"][0]["contexts"].as_array().unwrap().iter())
        .collect::<Vec<_>>();

    let described = contexts
        .iter()
        .find(|context| context["description"].is_string())
        .unwrap();
    assert!(described["description"].as_str().unwrap().contains("Any animal"));
    assert!(!described["meanings"].as_array().unwrap().is_empty());
}

#[test]
fn pronunciation_gives_uk_and_us_with_ipa_and_audio_path() {
    let definition = parse_fixture("beautiful");
    let pronounces = definition["classes"][0]["pronounces"].as_array().unwrap();

    assert_eq!(pronounces[0]["region"], "UK");
    assert_eq!(pronounces[0]["ipa"], "/ˈbjuːtɪ.fəl/");
    assert_eq!(
        pronounces[0]["link"],
        "/wikipedia/commons/transcoded/e/ea/En-uk-beautiful.ogg/En-uk-beautiful.ogg.mp3"
    );
    assert_eq!(pronounces[1]["region"], "US");
    assert!(pronounces[1]["link"].as_str().unwrap().ends_with("En-us-beautiful.ogg.mp3"));
}

#[test]
fn every_class_of_the_page_gets_the_page_pronunciation() {
    let definition = parse_fixture("beautiful");

    assert_eq!(
        definition["classes"][0]["pronounces"],
        definition["classes"][1]["pronounces"]
    );
}

#[test]
fn relation_lists_are_sections_of_their_class() {
    let definition = parse_fixture("beautiful");

    let synonyms = section(&definition, "synonym", Some("adjective"));
    assert_eq!(synonyms["label"], "Synonyms");
    assert!(item_texts(synonyms).iter().any(|text| text == "attractive"));
    assert!(!item_texts(section(&definition, "antonym", Some("adjective"))).is_empty());
    assert!(!item_texts(section(&definition, "derived", Some("adjective"))).is_empty());
}

#[test]
fn etymology_text_is_a_section_without_class() {
    let definition = parse_fixture("beautiful");
    let etymology = section(&definition, "etymology", None);

    assert!(item_texts(etymology)[0].starts_with("From "));
}

#[test]
fn each_numbered_etymology_is_its_own_section() {
    let definition = parse_fixture("abandon");
    let labels = definition["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|section| section["key"] == "etymology")
        .map(|section| section["label"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["Etymology 1", "Etymology 2"]);
}

#[test]
fn a_list_line_with_several_words_gives_one_item_per_word() {
    let definition = parse_fixture("beautiful");
    let texts = item_texts(section(&definition, "synonym", Some("adjective")));

    assert!(texts.contains(&"beauteous".to_owned()));
    assert!(texts.contains(&"attractive".to_owned()));
    assert!(texts.iter().all(|text| !text.contains(',')));
}

#[test]
fn list_items_keep_the_word_link() {
    let definition = parse_fixture("beautiful");
    let synonyms = section(&definition, "synonym", Some("adjective"));

    assert!(synonyms["items"].as_array().unwrap().iter().any(|item| item["link"] == "good-looking"));
}

#[test]
fn skipped_headings_do_not_become_sections() {
    let definition = parse_fixture("abandon");
    let labels = definition["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|section| section["label"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();

    assert!(!labels.contains(&"References".to_owned()));
    assert!(!labels.contains(&"Translations".to_owned()));
}

#[test]
fn a_page_without_an_english_section_is_not_found() {
    let error = parse("<section><h2 id=\"French\">French</h2><p>mot</p></section>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn an_english_section_without_definitions_is_not_found() {
    let error = parse("<section><h2 id=\"English\">English</h2><section><h3 id=\"Noun\">Noun</h3><ol></ol></section></section>").unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn the_kept_fragment_is_the_english_section_without_markup_noise() {
    let html = std::fs::read_to_string("tests/fixtures/wiktionary_beautiful.html").unwrap();
    let definition = parse(&html).unwrap();
    let fragment = definition.extracted_html.unwrap();

    assert!(fragment.starts_with("<section"));
    assert!(fragment.contains("id=\"Adjective\""));
    assert!(!fragment.contains("data-mw"));
    assert!(!fragment.contains("<script"));
    assert!(fragment.len() < html.len() + 1000);
}

#[test]
fn the_kept_fragment_parses_to_the_same_definition() {
    let html = std::fs::read_to_string("tests/fixtures/wiktionary_fish.html").unwrap();
    let first = parse(&html).unwrap();
    let again = parse(&first.extracted_html.clone().unwrap()).unwrap();

    assert_eq!(serde_json::to_value(&first).unwrap(), serde_json::to_value(&again).unwrap());
}
