use crate::scraper::scraper::WordDefinition;

pub trait HtmlParser {
    type Output;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output>;
}

pub struct CambridgeHtmlParser {}

impl CambridgeHtmlParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlParser for CambridgeHtmlParser {
    type Output = WordDefinition;
    fn parse(&self, html_content: &str) -> anyhow::Result<Self::Output> {
        todo!()
    }
}
