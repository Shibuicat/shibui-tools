pub mod scraper;
mod utils;

use anyhow::Result;
use clap::{ArgEnum, Parser};
use scraper::cambridge_dictionary_scraper::CambridgeDictionaryScraper;
use utils::{html_parser::CambridgeHtmlParser, http_request::DefaultHttpRequestMaker};

#[tokio::main]
async fn main() -> Result<()> {
    let request_maker = DefaultHttpRequestMaker::new();
    let html_parser = CambridgeHtmlParser::new();
    let fetcher = CambridgeDictionaryScraper::new(request_maker, html_parser);
    Ok(())
}

fn get_words() -> anyhow::Result<Vec<String>> {
    Ok(vec![])
}

pub struct Args {
    fetch_type: FetchType,
}

#[derive(Debug)]
#[command(version, about, long_about = None)]
pub enum FetchType {
    File(String),
    Word(String),
}

impl Parser for FetchType {
    fn parse() -> Self {
        let mut matches = <Self as clap::CommandFactory>::command().get_matches();
        let res = <Self as clap::FromArgMatches>::from_arg_matches_mut(&mut std::matches)
            .map_err(format_error::<Self>);
        match res {
            Ok(s) => s,
            Err(e) => {
                // Since this is more of a development-time error, we aren't doing as fancy of a quit
                // as `get_matches`
                e.exit()
            }
        }
    }

    fn try_parse() -> std::result::Result<Self, clap::Error> {
        let mut matches = ok!(<Self as CommandFactory>::command().try_get_matches());
        <Self as clap::FromArgMatches>::from_arg_matches_mut(&mut std::matches)
            .map_err(format_error::<Self>)
    }

    fn parse_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let mut matches = <Self as clap::CommandFactory>::command().get_matches_from(itr);
        let res = <Self as clap::FromArgMatches>::from_arg_matches_mut(&mut std::matches)
            .map_err(format_error::<Self>);
        match res {
            Ok(s) => s,
            Err(e) => {
                // Since this is more of a development-time error, we aren't doing as fancy of a quit
                // as `get_matches_from`
                e.exit()
            }
        }
    }

    fn try_parse_from<I, T>(itr: I) -> std::result::Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let mut matches = ok!(<Self as CommandFactory>::command().try_get_matches_from(itr));
        <Self as clap::FromArgMatches>::from_arg_matches_mut(&mut std::matches)
            .map_err(format_error::<Self>)
    }

    fn update_from<I, T>(&mut self, itr: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let mut matches =
            <Self as clap::CommandFactory>::command_for_update().get_matches_from(itr);
        let res =
            <Self as clap::FromArgMatches>::update_from_arg_matches_mut(self, &mut std::matches)
                .map_err(format_error::<Self>);
        if let Err(e) = res {
            // Since this is more of a development-time error, we aren't doing as fancy of a quit
            // as `get_matches_from`
            e.exit()
        }
    }

    fn try_update_from<I, T>(&mut self, itr: I) -> std::result::Result<(), clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let mut matches =
            ok!(<Self as CommandFactory>::command_for_update().try_get_matches_from(itr));
        <Self as clap::FromArgMatches>::update_from_arg_matches_mut(self, &mut std::matches)
            .map_err(format_error::<Self>)
    }
}
