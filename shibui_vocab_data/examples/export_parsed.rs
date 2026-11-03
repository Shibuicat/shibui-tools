use std::io::Write;
use std::path::{Path, PathBuf};

use shibui_vocab_data::utils::html_parser::{cambridge_parser::CambridgeHtmlParser, HtmlParser};

fn html_files(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| path.extension().is_some_and(|extension| extension == "html"))
                .collect()
        })
        .unwrap_or_default();
    paths.sort();
    paths
}

fn line_of(path: &Path) -> Result<String, String> {
    let word = path.file_stem().map(|stem| stem.to_string_lossy().to_string()).unwrap_or_default();
    let html = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let definition = CambridgeHtmlParser.parse(&html).map_err(|error| error.to_string())?;
    let line = serde_json::json!({ "word": word, "definition": definition });
    serde_json::to_string(&line).map_err(|error| error.to_string())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let usage = "usage: export_parsed <html_dir> <out_file.jsonl> [limit]";
    let dir = args.next().expect(usage);
    let out = args.next().expect(usage);
    let limit = args.next().and_then(|limit| limit.parse::<usize>().ok()).unwrap_or(usize::MAX);

    let mut writer = std::io::BufWriter::new(std::fs::File::create(&out).expect("cannot create the output file"));
    let (mut written, mut failed) = (0usize, 0usize);
    for path in html_files(Path::new(&dir)).into_iter().take(limit) {
        match line_of(&path) {
            Ok(line) => {
                writeln!(writer, "{line}").expect("cannot write the output file");
                written += 1;
            }
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                failed += 1;
            }
        }
    }
    writer.flush().expect("cannot write the output file");
    println!("{written} written, {failed} failed");
}
