use shibui_vocab_data::utils::html_parser::{
    merriam_webster_parser::MerriamWebsterHtmlParser, oxford_parser::OxfordHtmlParser,
    wiktionary_parser::WiktionaryHtmlParser, HtmlParser,
};
use shibui_vocab_data::scraper::WordDefinition;

fn parse(source: &str, html: &str) -> anyhow::Result<WordDefinition> {
    match source {
        "merriam-webster" => MerriamWebsterHtmlParser.parse(html),
        "oxford" => OxfordHtmlParser.parse(html),
        "wiktionary" => WiktionaryHtmlParser.parse(html),
        other => anyhow::bail!("unknown source {other}"),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let source = args.next().expect("usage: parse_pages <source> <pages_dir> <out_dir>");
    let pages = args.next().expect("usage: parse_pages <source> <pages_dir> <out_dir>");
    let out = args.next().expect("usage: parse_pages <source> <pages_dir> <out_dir>");
    std::fs::create_dir_all(&out).unwrap();

    let mut paths: Vec<_> = std::fs::read_dir(&pages).unwrap().map(|entry| entry.unwrap().path()).collect();
    paths.sort();
    let (mut parsed, mut failed) = (0, 0);
    for path in paths {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let html = std::fs::read_to_string(&path).unwrap();
        match parse(&source, &html) {
            Ok(definition) => {
                parsed += 1;
                std::fs::write(format!("{out}/{name}.json"), serde_json::to_string_pretty(&definition).unwrap()).unwrap();
                if let Some(extracted) = &definition.extracted_html {
                    std::fs::write(format!("{out}/{name}.extracted.html"), extracted).unwrap();
                }
            }
            Err(error) => {
                failed += 1;
                println!("{name}: {error}");
            }
        }
    }
    println!("{source}: {parsed} parsed, {failed} failed");
}
