use shibui_vocab_data::utils::html_parser::{cambridge_parser::CambridgeHtmlParser, HtmlParser};
use std::path::Path;

fn pronounces_by_class(html: &str) -> Option<serde_json::Value> {
    std::panic::catch_unwind(|| CambridgeHtmlParser.parse(html))
        .ok()
        .and_then(Result::ok)
        .map(|definition| {
            let json = serde_json::to_value(&definition).unwrap();
            json["classes"]
                .as_array()
                .map(|classes| {
                    classes
                        .iter()
                        .map(|class| {
                            serde_json::json!({
                                "name": class["className"],
                                "pronounces": class["pronounces"],
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
}

fn reparse(word: &str, dir: &Path) -> serde_json::Value {
    let parsed = std::fs::read_to_string(dir.join(format!("{word}.html")))
        .ok()
        .and_then(|html| pronounces_by_class(&html));
    serde_json::json!({ "word": word, "classes": parsed })
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let dir = std::env::args().nth(1).expect("usage: reparse_pronounces <html_dir> > out.jsonl");
    let mut words: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().to_str()?.strip_suffix(".html").map(String::from))
        .collect();
    words.sort();
    words
        .iter()
        .for_each(|word| println!("{}", reparse(word, Path::new(&dir))));
}
