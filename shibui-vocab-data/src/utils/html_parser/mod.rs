mod cambridge_elements;
pub mod cambridge_parser;

pub trait HtmlParser {
    type Output;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output>;
}
