use shibui_vocab_data::utils::html_parser::{
    cambridge_parser::CambridgeHtmlParser, oxford_parser::OxfordHtmlParser,
    wiktionary_parser::WiktionaryHtmlParser, HtmlParser,
};

fn dump(parser: &dyn Fn(&str) -> anyhow::Result<serde_json::Value>, dir: &str, out: &str) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let html = std::fs::read_to_string(&path).unwrap();
        let text = match parser(&html) {
            Ok(value) => serde_json::to_string_pretty(&value).unwrap(),
            Err(error) => format!("ERROR {error}"),
        };
        std::fs::write(format!("{out}/{name}.json"), text).unwrap();
    }
}


fn main() {
    let out = std::env::var("BASELINE_OUT").unwrap();
    let pages = std::env::var("PAGES_DIR").unwrap();
    std::fs::create_dir_all(format!("{out}/oxford")).unwrap();
    std::fs::create_dir_all(format!("{out}/wiktionary")).unwrap();
    std::fs::create_dir_all(format!("{out}/cambridge")).unwrap();
    dump(&|h| Ok(serde_json::to_value(OxfordHtmlParser.parse(h)?)?), &format!("{pages}/oxford"), &format!("{out}/oxford"));
    dump(&|h| Ok(serde_json::to_value(WiktionaryHtmlParser.parse(h)?)?), &format!("{pages}/wiktionary"), &format!("{out}/wiktionary"));
    dump(&|h| Ok(serde_json::to_value(CambridgeHtmlParser.parse(h)?)?), &std::env::var("CAMBRIDGE_DIR").unwrap(), &format!("{out}/cambridge"));
}
