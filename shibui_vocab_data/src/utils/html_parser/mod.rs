mod cambridge_elements;
pub mod cambridge_parser;

pub use cambridge_elements::WordNotFoundError;

pub trait HtmlParser {
    type Output;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output>;
}
