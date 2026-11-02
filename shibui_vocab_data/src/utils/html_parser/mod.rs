mod cambridge_elements;
pub mod cambridge_parser;
mod html_dom;
mod merriam_webster_fragment;
pub mod merriam_webster_parser;
mod merriam_webster_sections;
mod merriam_webster_senses;
mod oxford_fragment;
pub mod oxford_parser;
mod oxford_pronunciation;
mod oxford_sections;
mod oxford_senses;
mod wiktionary_fragment;
mod wiktionary_pronunciation;
mod wiktionary_senses;
pub mod wiktionary_parser;

pub use cambridge_elements::WordNotFoundError;

pub trait HtmlParser {
    type Output;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output>;
}
