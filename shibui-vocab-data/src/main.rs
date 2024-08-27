pub mod scraper;
use scraper::scraper::Scraper;
mod utils;

// use std::path::PathBuf;

use anyhow::Result;
// use clap::{Parser, Subcommand};
use scraper::cambridge_dictionary_scraper::CambridgeDictionaryScraper;
use utils::{
    html_parser::cambridge_parser::CambridgeHtmlParser, http_request::DefaultHttpRequestMaker,
};

#[tokio::main]
async fn main() -> Result<()> {
    // let args = Args::parse();
    // dbg!(args);
    let request_maker = DefaultHttpRequestMaker::new();
    let html_parser = CambridgeHtmlParser::new();
    let fetcher = CambridgeDictionaryScraper::new(request_maker, html_parser);
    fetcher.fetch("hello").await?;
    Ok(())
}

// fn get_words(path: &PathBuf) -> anyhow::Result<Vec<String>> {
//     Ok(vec![])
// }

// #[derive(Debug, Parser)]
// #[command(version, about, long_about = None)]
// pub struct Args {
//     #[command(subcommand)]
//     command: FetchType,
// }
//
// #[derive(Subcommand, Debug)]
// pub enum FetchType {
//     File { file_name: String },
//     Word { word: String },
// }
