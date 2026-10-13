use shibui_vocab_data::utils::html_parser::{
    oxford_parser::OxfordHtmlParser, HtmlParser, WordNotFoundError,
};

fn parse_fixture(name: &str) -> serde_json::Value {
    let html = std::fs::read_to_string(format!("tests/fixtures/oxford_{name}.html")).unwrap();
    serde_json::to_value(OxfordHtmlParser.parse(&html).unwrap()).unwrap()
}

fn contexts(definition: &serde_json::Value) -> &Vec<serde_json::Value> {
    definition["classes"][0]["definitions"][0]["contexts"].as_array().unwrap()
}

fn section<'a>(definition: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    definition["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["key"] == key)
        .unwrap_or_else(|| panic!("no section {key}"))
}

fn item_texts(section: &serde_json::Value) -> Vec<String> {
    section["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["text"].as_str().unwrap().to_owned())
        .collect()
}

fn has_section(definition: &serde_json::Value, key: &str) -> bool {
    definition["sections"].as_array().unwrap().iter().any(|section| section["key"] == key)
}

#[test]
fn the_headword_and_the_part_of_speech_are_read() {
    let definition = parse_fixture("abandon");

    assert_eq!(definition["word"], "abandon");
    assert_eq!(definition["classes"].as_array().unwrap().len(), 1);
    assert_eq!(definition["classes"][0]["className"], "Verb");
}

#[test]
fn uk_and_us_pronunciations_carry_ipa_and_a_host_less_audio_path() {
    let definition = parse_fixture("accomplish");

    let pronounces = definition["classes"][0]["pronounces"].as_array().unwrap();
    assert_eq!(pronounces.len(), 2);
    assert_eq!(pronounces[0]["region"], "UK");
    assert_eq!(pronounces[0]["ipa"], "/əˈkʌmplɪʃ/");
    assert_eq!(pronounces[1]["region"], "US");
    assert_eq!(pronounces[1]["ipa"], "/əˈkɑːmplɪʃ/");
    assert!(pronounces[0]["link"].as_str().unwrap().starts_with("/media/english/uk_pron/"));
    assert!(pronounces[1]["link"].as_str().unwrap().ends_with(".mp3"));
}

#[test]
fn each_sense_becomes_a_context_with_its_definition_and_examples() {
    let definition = parse_fixture("abandon");

    assert_eq!(contexts(&definition).len(), 5);
    let first = &contexts(&definition)[0]["meanings"][0];
    assert_eq!(
        first["explanation"],
        "to leave somebody, especially somebody you are responsible for, with no intention of returning"
    );
    assert_eq!(first["examples"][0], "The baby had been abandoned by its mother.");
    assert_eq!(first["examples"].as_array().unwrap().len(), 3);
}

#[test]
fn usage_patterns_grammar_and_labels_describe_the_sense() {
    let definition = parse_fixture("bear");

    let described = contexts(&definition)
        .iter()
        .filter_map(|context| context["description"].as_str())
        .collect::<Vec<_>>();
    assert!(described.contains(&"support weight [transitive] bear something"));
}

#[test]
fn senses_grouped_under_a_short_heading_keep_the_heading_as_the_description() {
    let definition = parse_fixture("cold");

    assert_eq!(contexts(&definition)[0]["description"], "low temperature");
    assert_eq!(contexts(&definition)[1]["description"], "food/drink");
    assert_eq!(contexts(&definition).len(), 8);
}

#[test]
fn a_sense_without_notes_has_no_description() {
    let definition = parse_fixture("although");

    assert!(contexts(&definition)[0]["description"].is_null());
}

#[test]
fn the_verb_forms_table_becomes_a_section() {
    let definition = parse_fixture("abandon");

    assert_eq!(
        item_texts(section(&definition, "verb-form")),
        vec![
            "present simple I / you / we / they: abandon",
            "he / she / it: abandons",
            "past simple: abandoned",
            "past participle: abandoned",
            "-ing form: abandoning",
        ]
    );
}

#[test]
fn levels_word_lists_and_topics_are_kept() {
    let definition = parse_fixture("abandon");

    assert_eq!(item_texts(section(&definition, "level"))[0], "Headword: B2");
    assert!(item_texts(section(&definition, "level")).contains(&"Sense 2: B2".to_owned()));
    assert_eq!(item_texts(section(&definition, "word-list")), vec!["Oxford 3000"]);
    let topic = &section(&definition, "topic")["items"][0];
    assert_eq!(topic["text"], "Difficulty and failure (b2)");
    assert_eq!(topic["link"], "/topic/difficulty-and-failure?level=b2");
}

#[test]
fn the_headword_level_is_not_mixed_with_sense_levels() {
    let definition = parse_fixture("cold");

    let levels = item_texts(section(&definition, "level"));
    assert_eq!(levels.iter().filter(|level| level.starts_with("Headword")).count(), 1);
}

#[test]
fn synonym_references_link_to_the_word_without_its_entry_number() {
    let definition = parse_fixture("abandon");

    let synonym = &section(&definition, "synonym")["items"][0];
    assert_eq!(synonym["text"], "leave");
    assert_eq!(synonym["link"], "leave");
}

#[test]
fn see_also_references_are_kept_apart_from_synonyms() {
    let definition = parse_fixture("cold");

    assert!(item_texts(section(&definition, "see-also")).contains(&"stone cold".to_owned()));
    assert!(!has_section(&definition, "synonym"));
}

#[test]
fn word_origin_collocations_and_extra_examples_are_kept() {
    let definition = parse_fixture("abandon");

    assert!(item_texts(section(&definition, "etymology"))[0].starts_with("late Middle English"));
    let collocations = item_texts(section(&definition, "collocation"));
    assert!(collocations.contains(&"abandon somebody to their fate".to_owned()));
    assert!(!collocations.contains(&"…".to_owned()));
    assert!(item_texts(section(&definition, "extra-example")).contains(&"The town had been hastily abandoned.".to_owned()));
}

#[test]
fn inflections_variants_and_idioms_are_kept() {
    assert_eq!(
        item_texts(section(&parse_fixture("cold"), "inflection")),
        vec!["comparative colder", "superlative coldest"]
    );
    assert_eq!(
        item_texts(section(&parse_fixture("although"), "alternative-form")),
        vec!["US English, informal altho"]
    );
    let idioms = item_texts(section(&parse_fixture("back"), "idiom"));
    assert!(idioms[0].starts_with("at/in the back of your mind: if a thought"));
}

#[test]
fn idioms_are_not_mistaken_for_senses_of_the_word() {
    let definition = parse_fixture("back");

    assert!(contexts(&definition).iter().all(|context| {
        !context["meanings"][0]["explanation"].as_str().unwrap().starts_with("if a thought, etc. is")
    }));
}

#[test]
fn phrasal_verbs_link_to_their_own_word() {
    let definition = parse_fixture("bear");

    let items = &section(&definition, "phrasal-verb")["items"];
    assert_eq!(items[0]["text"], "bear down on");
    assert_eq!(items[0]["link"], "bear down on");
}

#[test]
fn boxes_with_free_text_are_kept_as_one_readable_item() {
    let definition = parse_fixture("back");

    let text = &item_texts(section(&definition, "which-word"))[0];
    assert!(text.starts_with("at the back / at the rear / behind"));
}

#[test]
fn the_kept_fragment_has_no_ads_images_or_app_banners() {
    let html = std::fs::read_to_string("tests/fixtures/oxford_abandon.html")
        .unwrap()
        .replace("<div class=\"entry\"", "<img src=\"x.png\"><div id=\"ring-links-box\">ad</div><div class=\"entry\"");
    assert!(html.contains("ring-links-box"));

    let fragment = OxfordHtmlParser.parse(&html).unwrap().extracted_html.unwrap();

    assert!(fragment.starts_with("<div") && fragment.contains("id=\"entryContent\""));
    assert!(!fragment.contains("<img"));
    assert!(!fragment.contains("ring-links-box"));
    assert!(fragment.contains("data-src-mp3"));
}

#[test]
fn the_kept_fragment_parses_again_to_the_same_word() {
    let html = std::fs::read_to_string("tests/fixtures/oxford_abandon.html").unwrap();
    let fragment = OxfordHtmlParser.parse(&html).unwrap().extracted_html.unwrap();

    let again = OxfordHtmlParser.parse(&fragment).unwrap();

    assert_eq!(again.word, "abandon");
    assert_eq!(again.classes[0].definitions[0].contexts.len(), 5);
}

#[test]
fn the_spelling_suggestion_page_is_not_found() {
    let html = std::fs::read_to_string("tests/fixtures/oxford_not_found.html").unwrap();

    let error = OxfordHtmlParser.parse(&html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_some());
}

#[test]
fn an_unknown_layout_is_a_failure_not_a_not_found() {
    let html = std::fs::read_to_string("tests/fixtures/oxford_layout_changed.html").unwrap();

    let error = OxfordHtmlParser.parse(&html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_none());
}

#[test]
fn an_entry_without_any_sense_is_a_failure() {
    let html = "<html><head><title>x</title></head><body><div id=\"entryContent\"><div class=\"entry\"><div class=\"top-container\"><div class=\"top-g\"><div class=\"webtop\"><h1 class=\"headword\">x</h1><span class=\"pos\">noun</span></div></div></div></div></div></body></html>";

    let error = OxfordHtmlParser.parse(html).unwrap_err();

    assert!(error.downcast_ref::<WordNotFoundError>().is_none());
}

#[test]
fn the_homograph_number_is_not_part_of_the_word() {
    assert_eq!(parse_fixture("close")["word"], "close");
}

#[test]
fn oxford_5000_levels_and_opal_lists_are_kept() {
    let accomplish = parse_fixture("accomplish");
    assert!(item_texts(section(&accomplish, "level"))[0].starts_with("Headword: "));
    assert_eq!(item_texts(section(&accomplish, "word-list")), vec!["Oxford 5000"]);

    let although = parse_fixture("although");
    let lists = item_texts(section(&although, "word-list"));
    assert!(lists.contains(&"OPAL Written".to_owned()));
    assert!(lists.contains(&"OPAL Spoken".to_owned()));
}

#[test]
fn the_word_family_box_is_kept_as_a_list() {
    let definition = parse_fixture("happy");

    let family = item_texts(section(&definition, "word-family"));
    assert!(family.contains(&"happy adjective (≠ unhappy)".to_owned()));
    assert!(family.contains(&"happily adverb (≠ unhappily)".to_owned()));
}

#[test]
fn a_box_the_parser_does_not_know_is_kept_under_its_own_title() {
    let html = std::fs::read_to_string("tests/fixtures/oxford_abandon.html").unwrap().replace(
        "<ol class=\"senses_multiple\"",
        "<span class=\"unbox\" unbox=\"newbox\"><span class=\"box_title\">Brand New <span class=\"closed\">x</span></span><span class=\"body\"><span class=\"p\">Some text</span></span></span><ol class=\"senses_multiple\"",
    );
    assert!(html.contains("newbox"));

    let definition = serde_json::to_value(OxfordHtmlParser.parse(&html).unwrap()).unwrap();

    let kept = section(&definition, "newbox");
    assert_eq!(kept["label"], "Brand New");
    assert_eq!(item_texts(kept), vec!["Some text"]);
}
