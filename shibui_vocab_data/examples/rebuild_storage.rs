use shibui_vocab_data::utils::html_parser::{cambridge_parser::CambridgeHtmlParser, HtmlParser};
use std::{io::BufRead, path::Path};

fn extract(html: &str) -> Option<String> {
    std::panic::catch_unwind(|| CambridgeHtmlParser.parse(html))
        .ok()
        .and_then(Result::ok)
        .and_then(|definition| definition.extracted_html)
}

fn rebuild(word: &str, in_dir: &Path, out_dir: &Path) -> &'static str {
    let Ok(html) = std::fs::read_to_string(in_dir.join(format!("{word}.html"))) else {
        return "nofile";
    };
    match extract(&html) {
        None => "unparsed",
        Some(trimmed) => match std::fs::write(out_dir.join(format!("{word}.html")), trimmed) {
            Ok(()) => "written",
            Err(_) => "writefailed",
        },
    }
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut args = std::env::args().skip(1);
    let in_dir = args.next().expect("usage: rebuild_storage <in_dir> <out_dir> < words");
    let out_dir = args.next().expect("usage: rebuild_storage <in_dir> <out_dir> < words");
    std::fs::create_dir_all(&out_dir).unwrap();
    for word in std::io::stdin().lock().lines().map(Result::unwrap) {
        let outcome = rebuild(&word, Path::new(&in_dir), Path::new(&out_dir));
        println!("R\t{word}\t{outcome}");
    }
}
