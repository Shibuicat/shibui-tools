pub trait HtmlParser {
    fn parse<T>(&self, html_content: &str) -> anyhow::Result<T>;
}

pub struct DefaultHtmlParser {}

impl HtmlParser for DefaultHtmlParser {
    fn parse<T>(&self, html_content: &str) -> anyhow::Result<T> {
        todo!()
    }
}
