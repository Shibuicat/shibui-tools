use serde_json::{json, Value};
use shibui_vocab_data::scraper::{
    Class, ClassDefinition, ExampleSource, Group, Label, LinkKind, Region, Relation, RelationTerm,
    Section, SectionItem, Topic, WordClass, WordContext, WordDefinition, WordExplanation,
    WordForm, WordPronounce, WordUsageExample,
};

fn keys(value: &Value) -> Vec<String> {
    let mut keys = value.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    keys.sort();
    keys
}

fn pronunciation() -> WordPronounce {
    WordPronounce {
        region: Region::UK,
        ipa: "/əˈbæn.dən/".to_owned(),
        link: "/media/abandon.mp3".to_owned(),
    }
}

fn plain_definition() -> WordDefinition {
    let meaning = WordExplanation::new(
        "to leave a place".to_owned(),
        vec![WordUsageExample::plain("We had to abandon the car.".to_owned())],
    );
    WordDefinition::new(
        "abandon".to_owned(),
        vec![WordClass::new(
            Class::Verb,
            vec![ClassDefinition {
                contexts: vec![WordContext {
                    description: Some("LEAVE".to_owned()),
                    meanings: vec![meaning],
                }],
            }],
            vec![pronunciation()],
        )],
        Vec::new(),
    )
}

#[test]
fn a_plain_definition_has_only_the_original_fields() {
    let value = serde_json::to_value(plain_definition()).unwrap();

    assert_eq!(keys(&value), ["classes", "word"]);
    assert_eq!(keys(&value["classes"][0]), ["className", "definitions", "pronunciations"]);
    let meaning = &value["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0];
    assert_eq!(keys(meaning), ["examples", "explanation"]);
}

#[test]
fn a_plain_example_is_an_object_with_only_its_text() {
    let value = serde_json::to_value(plain_definition()).unwrap();
    let meaning = &value["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0];

    assert_eq!(meaning["examples"], json!([{ "text": "We had to abandon the car." }]));
}

#[test]
fn the_pronunciation_list_is_named_pronunciations() {
    let value = serde_json::to_value(plain_definition()).unwrap();

    assert!(value["classes"][0].get("pronounces").is_none());
    assert_eq!(
        value["classes"][0]["pronunciations"],
        json!([{ "region": "UK", "ipa": "/əˈbæn.dən/", "link": "/media/abandon.mp3" }])
    );
}

#[test]
fn every_new_field_is_written_in_camel_case_when_present() {
    let mut meaning = WordExplanation::new("to give up".to_owned(), Vec::new());
    meaning.grammar = Some("transitive".to_owned());
    meaning.level = Some("B2".to_owned());
    meaning.pattern = Some("[VERB noun]".to_owned());
    meaning.notes = vec!["often passive".to_owned()];
    meaning.labels = vec![Label { kind: "register".to_owned(), text: "formal".to_owned() }];
    meaning.topics = vec![Topic { text: "Law".to_owned(), level: Some("C1".to_owned()), link: None }];
    meaning.relations = vec![Relation {
        kind: "antonym".to_owned(),
        terms: vec![RelationTerm { text: "keep".to_owned(), link: Some("keep".to_owned()), relevance: None }],
    }];
    let mut sub_meaning = WordExplanation::new("to give up control".to_owned(), Vec::new());
    sub_meaning.examples = vec![WordUsageExample {
        text: "He abandoned himself.".to_owned(),
        pattern: Some("[VERB oneself]".to_owned()),
        gloss: Some("gave in".to_owned()),
        source: Some(ExampleSource {
            title: Some("Barron's".to_owned()),
            author: Some("A. Writer".to_owned()),
            date: Some("Oct. 9, 2026".to_owned()),
            link: Some("https://example.org/a".to_owned()),
        }),
        translation: Some("se laissa aller".to_owned()),
    }];
    meaning.sub_meanings = vec![sub_meaning];

    let mut class = WordClass::new(
        Class::Verb,
        vec![ClassDefinition {
            contexts: vec![WordContext { description: None, meanings: vec![meaning] }],
        }],
        Vec::new(),
    );
    class.sub_class = Some("transitive verb".to_owned());
    class.grammar = Some("[T]".to_owned());
    class.level = Some("B2".to_owned());
    class.forms = vec![WordForm {
        label: Some("past tense".to_owned()),
        text: "abandoned".to_owned(),
        audio: Some("/media/abandoned.mp3".to_owned()),
        pronunciations: vec![pronunciation()],
    }];

    let mut definition = WordDefinition::new("abandon".to_owned(), vec![class], Vec::new());
    definition.groups = vec![Group {
        key: "collins-english".to_owned(),
        kind: "dictionary".to_owned(),
        label: "Collins English Dictionary".to_owned(),
        attribution: Some("Copyright HarperCollins".to_owned()),
        frequency: Some(4),
        pronunciations: vec![pronunciation()],
        classes: Vec::new(),
        sections: vec![Section {
            key: Some("etymology".to_owned()),
            label: "Etymology".to_owned(),
            class_name: None,
            items: vec![SectionItem {
                text: "from Old French".to_owned(),
                link: Some("abandoner".to_owned()),
                link_kind: Some(LinkKind::Word),
                label: Some("verb".to_owned()),
                description: Some("origin".to_owned()),
                examples: vec![WordUsageExample::plain("an example".to_owned())],
                audio: Some("/media/x.mp3".to_owned()),
            }],
        }],
    }];

    let value = serde_json::to_value(definition).unwrap();

    assert_eq!(keys(&value), ["classes", "groups", "word"]);
    assert_eq!(
        keys(&value["classes"][0]),
        ["className", "definitions", "forms", "grammar", "level", "pronunciations", "subClass"]
    );
    assert_eq!(
        keys(&value["classes"][0]["forms"][0]),
        ["audio", "label", "pronunciations", "text"]
    );
    let meaning = &value["classes"][0]["definitions"][0]["contexts"][0]["meanings"][0];
    assert_eq!(
        keys(meaning),
        ["examples", "explanation", "grammar", "labels", "level", "notes", "pattern", "relations", "subMeanings", "topics"]
    );
    let example = &meaning["subMeanings"][0]["examples"][0];
    assert_eq!(keys(example), ["gloss", "pattern", "source", "text", "translation"]);
    assert_eq!(keys(&example["source"]), ["author", "date", "link", "title"]);
    let group = &value["groups"][0];
    assert_eq!(
        keys(group),
        ["attribution", "classes", "frequency", "key", "kind", "label", "pronunciations", "sections"]
    );
    let item = &group["sections"][0]["items"][0];
    assert_eq!(keys(item), ["audio", "description", "examples", "label", "link", "linkKind", "text"]);
    assert_eq!(item["linkKind"], "word");
}

#[test]
fn a_definition_with_meanings_only_in_a_group_still_counts_as_having_meanings() {
    let mut definition = WordDefinition::new("abandon".to_owned(), Vec::new(), Vec::new());
    assert!(!definition.has_meanings());

    let with_meaning = plain_definition();
    definition.groups = vec![Group {
        key: "american".to_owned(),
        kind: "region".to_owned(),
        label: "American".to_owned(),
        attribution: None,
        frequency: None,
        pronunciations: Vec::new(),
        classes: with_meaning.classes,
        sections: Vec::new(),
    }];

    assert!(definition.has_meanings());
}
