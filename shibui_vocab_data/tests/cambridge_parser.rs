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
    assert_eq!(definition["classes"][0]["pronunciations"], serde_json::json!([]));
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
fn the_new_fixtures_also_parse_back_from_their_kept_fragment() {
    ["abandon", "bear", "brunt", "addenda"].iter().for_each(|name| {
        let html = std::fs::read_to_string(format!("tests/fixtures/cambridge_{name}.html")).unwrap();
        let first = parse(&html).unwrap();
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
    as_json(definition)["classes"][0]["pronunciations"].clone()
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

fn fixture(word: &str) -> serde_json::Value {
    let html = std::fs::read_to_string(format!("tests/fixtures/cambridge_{word}.html")).unwrap();
    as_json(&parse(&html).unwrap())
}

fn all_meanings(definition: &serde_json::Value) -> Vec<serde_json::Value> {
    let groups = definition["groups"].as_array().cloned().unwrap_or_default();
    std::iter::once(definition["classes"].clone())
        .chain(groups.into_iter().map(|group| group["classes"].clone()))
        .flat_map(|classes| classes.as_array().cloned().unwrap_or_default())
        .flat_map(|class| class["definitions"].as_array().cloned().unwrap_or_default())
        .flat_map(|definition| definition["contexts"].as_array().cloned().unwrap_or_default())
        .flat_map(|context| context["meanings"].as_array().cloned().unwrap_or_default())
        .collect()
}

fn entry_with_sense(definition_html: &str) -> String {
    format!(
        r#"<div class="pr entry-body__el"><div class="pos-header dpos-h"><div class="di-title"><span class="hw dhw">go</span></div><div class="posgram dpos-g"><span class="pos dpos">verb</span></div></div><div class="pos-body"><div class="pr dsense"><div class="sense-body dsense_b">{definition_html}</div></div></div></div>"#
    )
}

#[test]
fn the_american_and_business_dictionaries_are_groups_beside_the_primary_one() {
    let definition = fixture("abandon");

    assert_eq!(definition["classes"][0]["className"], "Verb");
    let groups = definition["groups"].as_array().unwrap();
    let described = groups
        .iter()
        .map(|group| format!("{}: {}", group["key"].as_str().unwrap(), group["label"].as_str().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(described, vec!["cacd: American English", "cbed: Business English"]);
    assert_eq!(groups[0]["kind"], "dictionary");
}

#[test]
fn level_grammar_and_the_guide_word_are_typed_fields() {
    let abandon = fixture("abandon");
    let context = &abandon["classes"][0]["definitions"][0]["contexts"][0];

    assert_eq!(context["description"], "LEAVE");
    assert_eq!(context["meanings"][0]["level"], "B2");
    assert_eq!(abandon["classes"][0]["grammar"], "[T]");
    assert_eq!(
        context["meanings"][0]["explanation"],
        "to leave a place, thing, or person, usually for ever"
    );
}

#[test]
fn the_more_examples_list_is_added_to_the_meaning_it_belongs_to() {
    let meaning = &fixture("abandon")["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0];

    assert_eq!(meaning["examples"].as_array().unwrap().len(), 9);
    assert_eq!(meaning["examples"][3]["pattern"], "abandon ship");
}

#[test]
fn example_patterns_and_glosses_are_separate_from_the_sentence() {
    let bear = fixture("bear");
    let with_gloss = all_meanings(&bear)
        .into_iter()
        .flat_map(|meaning| meaning["examples"].as_array().cloned().unwrap_or_default())
        .find(|example| example["pattern"] == "bear something in mind")
        .unwrap();

    assert_eq!(with_gloss["gloss"], "(= I will remember and consider it)");
    assert!(!with_gloss["text"].as_str().unwrap().contains("(="));
}

#[test]
fn a_grammar_code_inside_an_example_is_its_pattern() {
    let happy = fixture("happy");
    let patterns = all_meanings(&happy)
        .into_iter()
        .flat_map(|meaning| meaning["examples"].as_array().cloned().unwrap_or_default())
        .filter_map(|example| example["pattern"].as_str().map(str::to_owned))
        .collect::<Vec<_>>();

    assert!(patterns.contains(&"[+ to infinitive]".to_owned()));
}

#[test]
fn inflections_are_forms_with_their_label() {
    let forms = fixture("bear")["classes"][0]["forms"].clone();

    let described = forms
        .as_array()
        .unwrap()
        .iter()
        .map(|form| format!("{}{}", form["label"].as_str().map(|l| format!("{l} ")).unwrap_or_default(), form["text"].as_str().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(described, vec!["bore", "borne", "US also born"]);
}

#[test]
fn synonyms_and_opposites_are_relations_of_the_meaning() {
    let synonym = &fixture("absorptive")["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0]["relations"][0];
    assert_eq!(synonym["kind"], "synonym");
    assert_eq!(synonym["terms"][0]["text"], "absorbent");
    assert_eq!(synonym["terms"][0]["link"], "absorbent");

    let accurate = fixture("accurate");
    let opposite = all_meanings(&accurate)
        .into_iter()
        .flat_map(|meaning| meaning["relations"].as_array().cloned().unwrap_or_default())
        .find(|relation| relation["kind"] == "antonym")
        .unwrap();
    assert_eq!(opposite["terms"][0]["text"], "imprecise");
}

#[test]
fn a_usage_label_closing_the_definition_becomes_a_label() {
    let meaning = &fixture("addenda")["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0];

    assert_eq!(meaning["explanation"], "plural of addendum");
    assert_eq!(meaning["labels"][0]["kind"], "usage");
    assert_eq!(meaning["labels"][0]["text"], "specialized");
}

#[test]
fn an_entry_without_a_part_of_speech_gets_the_unknown_class() {
    assert_eq!(fixture("addenda")["classes"][0]["className"], "unknown");
}

#[test]
fn an_entry_that_only_defines_phrases_uses_them_as_its_meanings() {
    let brunt = fixture("brunt");
    let context = &brunt["classes"][0]["definitions"][0]["contexts"][0];

    assert_eq!(context["description"], "the brunt of something");
    assert_eq!(context["meanings"][0]["explanation"], "the main force of something unpleasant");
    assert!(brunt["sections"].as_array().unwrap().iter().all(|section| section["key"] != "phrase"));
}

#[test]
fn run_on_words_are_derived_forms_with_their_audio() {
    let american = &fixture("abandon")["groups"][0];
    let derived = american["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["key"] == "derived")
        .unwrap();

    assert_eq!(derived["items"][0]["text"], "abandonment");
    assert_eq!(derived["items"][0]["label"], "noun");
    assert!(derived["items"][0]["audio"].as_str().unwrap().ends_with("abandonment.mp3"));
}

#[test]
fn phrases_idioms_and_phrasal_verbs_are_sections_of_their_class() {
    let bear = fixture("bear");
    let sections = bear["sections"].as_array().unwrap();

    let phrasal = sections.iter().find(|s| s["key"] == "phrasal-verb").unwrap();
    assert_eq!(phrasal["className"], "verb");
    assert_eq!(phrasal["items"][0]["linkKind"], "word");
    assert!(sections.iter().any(|s| s["key"] == "idiom"));
}

#[test]
fn a_header_with_two_parts_of_speech_keeps_both() {
    let html = r#"<div class="pr entry-body__el"><div class="pos-header dpos-h"><div class="di-title"><span class="hw dhw">close</span></div><div class="posgram dpos-g"><span class="pos dpos">adjective</span>, <span class="pos dpos">adverb</span> <span class="gram dgram">[ <span class="gc dgc">-er/-est only</span> ]</span></div></div><div class="pos-body"><div class="pr dsense"><div class="sense-body dsense_b"><div class="def-block ddef_block"><div class="ddef_h"><div class="def ddef_d db">near:</div></div></div></div></div></div></div>"#;

    let class = &as_json(&parse(html).unwrap())["classes"][0];

    assert_eq!(class["className"], "Adjective");
    assert_eq!(class["subClass"], "adjective, adverb");
    assert_eq!(class["grammar"], "[-er/-est only]");
}

#[test]
fn the_thesaurus_box_is_a_relation_of_its_meaning_with_an_example_for_each_word() {
    let thesaurus = r#"<div class="def-block ddef_block"><div class="ddef_h"><div class="def ddef_d db">to go:</div></div><div class="def-body ddef_b"><div class="daccord fs16"><amp-accordion><section><header class="ca_h">Thesaurus: synonyms, antonyms, and examples</header><div class="daccord_b"><div class="daccord_lt"><a href="/thesaurus/articles/leave">to leave</a></div><div class="daccord_lb"><ul><li class="had t-i"><a href="/thesaurus/depart">depart</a><span class="ti lml-5"><span class="example dexample"><span class="lab dlab"><span class="region dregion">UK</span> </span>The train departs at noon.</span></span></li></ul></div></div></section></amp-accordion></div></div></div>"#;

    let definition = as_json(&parse(&entry_with_sense(thesaurus)).unwrap());

    let meaning = &definition["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0];
    assert_eq!(meaning["relations"][0]["kind"], "thesaurus");
    assert_eq!(meaning["relations"][0]["terms"][0]["text"], "depart");
    assert_eq!(meaning["relations"][0]["terms"][0]["examples"][0]["text"], "The train departs at noon.");
    assert!(definition["sections"].as_array().map_or(true, |sections| sections.iter().all(|s| s["key"] != "thesaurus")));
}

#[test]
fn a_pronunciation_with_a_nested_stress_mark_keeps_all_of_its_text() {
    let definition = activator_with_pronunciations(&[block(
        "uk",
        r#"æbˈdɒm.ɪ.n<span class="sp dsp">ə</span>l"#,
        "/media/english/uk_pron/a.mp3",
    )]);

    assert_eq!(pronounces(&definition)[0]["ipa"], "æbˈdɒm.ɪ.nəl");
}
