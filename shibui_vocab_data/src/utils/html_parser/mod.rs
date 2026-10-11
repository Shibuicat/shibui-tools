mod cambridge_elements;
pub mod cambridge_parser;
mod wiktionary_dom;
mod wiktionary_fragment;
mod wiktionary_pronunciation;
mod wiktionary_senses;
pub mod wiktionary_parser;

pub use cambridge_elements::WordNotFoundError;

pub trait HtmlParser {
    type Output;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output>;
}
